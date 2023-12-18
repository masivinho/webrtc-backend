## Run with

In a terminal: 

```
$ WEBSOCKET_PORT=8000 PUBLIC_WEBRTC_ADDR=127.0.0.1:8888 WEBRTC_ADDR=127.0.0.1:8888 RUST_LOG=info cargo run
```

Please note that if you are using Firefox, Firefox does not accept WebRTC
connections to 127.0.0.1, so you may need to use a different IP address.

## Build

```
$ bash scripts/build
```

You can find your binary on `target/release`

## Docker Compose

```
$ docker compose up -d
```

## License

This project is licensed under the [BSD license](LICENSE)