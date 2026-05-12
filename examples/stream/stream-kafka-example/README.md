## start redpanda

```sh
docker compose up -d
```

## run producer

```sh
cargo run --bin producer
```

access endpoint send msg:

```sh
curl http://localhost:8080
```

## run consumer

```sh
cargo run --bin consumer
```