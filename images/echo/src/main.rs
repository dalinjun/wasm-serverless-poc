use bytecodec::DecodeExt;
use httpcodec::{HeaderField, HttpVersion, ReasonPhrase, Response, Request, RequestDecoder, StatusCode};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time::Duration};

#[cfg(feature = "unix")]
use std::net::{Shutdown, TcpListener, TcpStream};

#[cfg(feature = "wasmedge")]
use wasmedge_wasi_socket::{Shutdown, TcpListener, TcpStream};

fn handle_http(req: Request<String>) -> bytecodec::Result<Response<String>> {
    let body = format!("{}", req.body());
    let length = body.len().to_string();

    println!("echoing back '{}'", body);

    let mut response = Response::new(
        HttpVersion::V1_1,
        StatusCode::new(200)?,
        ReasonPhrase::new("")?,
        body,
    );

    response.header_mut().add_field(HeaderField::new("content-length", length.as_str())?);

    Ok(response)
}

fn handle_healthcheck() -> bytecodec::Result<Response<String>> {
    println!("responding to healthcheck");

    Ok(Response::new(
        HttpVersion::V1_1,
        StatusCode::new(204)?,
        ReasonPhrase::new("")?,
        "".to_string(),
    ))
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buff = [0u8; 1024];
    let mut data = Vec::new();

    loop {
        let n = stream.read(&mut buff)?;
        data.extend_from_slice(&buff[0..n]);
        if n < 1024 {
            break;
        }
    }

    let mut decoder =
        RequestDecoder::<httpcodec::BodyDecoder<bytecodec::bytes::Utf8Decoder>>::default();

    let req = match decoder.decode_from_bytes(data.as_slice()) {
        Ok(req) => match req.request_target().as_str() {
            "/healthz" => handle_healthcheck(),
            _ => handle_http(req),
        },
        Err(e) => Err(e),
    };

    let r = match req {
        Ok(r) => r,
        Err(e) => {
            let err = format!("{:?}", e);
            Response::new(
                HttpVersion::V1_1,
                StatusCode::new(500).unwrap(),
                ReasonPhrase::new(err.as_str()).unwrap(),
                err.clone(),
            )
        }
    };

    let write_buf = r.to_string();
    stream.write(write_buf.as_bytes())?;
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let port = 8080;
    println!("serving at {}", port);

    #[cfg(feature = "unix")]
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;

    #[cfg(feature = "wasmedge")]
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port), true)?;

    #[cfg(feature = "unix")]
    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking");

    let shutdown = Arc::new(AtomicBool::new(false));

    #[cfg(feature = "unix")]
    let s = shutdown.clone();

    #[cfg(feature = "unix")]
    ctrlc::set_handler(move || {
        s.store(true, Ordering::SeqCst);
    })
    .expect("Error setting signal handler");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let _ = handle_client(s);
            }

            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                if shutdown.load(Ordering::SeqCst) {
                    println!("shutting down");
                    break;
                }

                thread::sleep(Duration::from_millis(1));

                continue;
            }

            Err(e) => panic!("encountered IO error: {}", e),
        }
    }

    Ok(())
}