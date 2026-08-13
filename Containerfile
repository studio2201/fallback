FROM registry.access.redhat.com/ubi9/ubi:latest AS builder
RUN dnf install -y gcc gcc-c++ make openssl-devel
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add wasm32-unknown-unknown
RUN curl -L https://github.com/trunk-rs/trunk/releases/download/v0.20.1/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf - -C /usr/local/bin

WORKDIR /app
COPY shared-assets /app/shared-assets
COPY fallback /app/fallback
WORKDIR /app/fallback

RUN trunk build --release

FROM registry.access.redhat.com/ubi9/ubi:latest
RUN dnf install -y nginx && dnf clean all
COPY --from=builder /app/fallback/dist /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
