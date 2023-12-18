FROM rust:1.74.1 as builder

WORKDIR /usr/src/app

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo,from=rust:1.74.1,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release && mv ./target/release/webrtc-backend ./webrtc-backend

FROM ubuntu:jammy

RUN apt-get update && apt install -y openssl

RUN useradd -ms /bin/bash app

USER app
WORKDIR /app

COPY --from=builder /usr/src/app/webrtc-backend /app/webrtc-backend
COPY ./configs/production.env /app/production.env

CMD /bin/bash -c "./webrtc-backend"