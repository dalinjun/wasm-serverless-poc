FROM docker.io/library/ubuntu:22.04 AS builder

# Install dependencies
RUN apt update && apt install -y autoconf curl git libcap-dev libseccomp-dev libsystemd-dev libtool libyajl-dev make pkg-config python3

# Install WasmEdge
ADD https://raw.githubusercontent.com/WasmEdge/WasmEdge/55dfc3d6f8671ec163addef3137824dae1c301de/utils/install.sh /wasmedge_install.sh
RUN chmod u+x /wasmedge_install.sh && ./wasmedge_install.sh --path="/usr/local" --version="0.10.1"

# Build crun
RUN git clone -b 1.5 https://github.com/containers/crun
WORKDIR /crun
RUN ./autogen.sh && ./configure --with-wasmedge && make

FROM scratch

COPY --from=builder /crun/crun /crun
