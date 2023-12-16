## Run with

In a terminal: 

```
$ PUBLIC_WEBRTC_ADDR=127.0.0.1:8888 WEBRTC_ADDR=127.0.0.1:8888 RUST_LOG=info cargo run
```

Please note that if you are using Firefox, Firefox does not accept WebRTC
connections to 127.0.0.1, so you may need to use a different IP address.

## Build

```
$ bash scripts/build
```

You can find your binary on `target/release`

## Docker

build the docker image

```
$ docker build -t webrtc_backend:latest .
```

create a new container

```
$ docker create -p42069:42069 -p8888:8888 --name webrtc_server webrtc_backend
```

run container

```
$ docker start webrtc_server
```

## License

This project is licensed under the [BSD license](LICENSE)