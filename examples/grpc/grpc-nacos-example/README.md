# grpc-nacos-example

gRPC server registered to Nacos via `summer-grpc` + `summer-nacos`, and a client that discovers the service by name.

## Prerequisites

```bash
cd examples/grpc/grpc-nacos-example
docker compose up -d
```

## Server

Registers `grpc-nacos-example` on [`ServerStartedEvent`](../../../summer/src/event.rs) with instance metadata `protocol=grpc` (port **9090**).

```bash
cargo run --bin server
```

Check the instance in the r-nacos console: `http://127.0.0.1:10848/rnacos/` (admin/admin).

## Client

Loads `config/client.toml`, lists all healthy instances for `discovery.service_name` (prefers `protocol=grpc`), and builds a single `GreeterClient` on a **tonic load-balanced `Channel`** (P2C across endpoints).

Start one or more servers (different ports via `GRPC_PORT` / config — see below), then:

```bash
cargo run --bin client
```

With a single server you will see one URI; with multiple instances registered in Nacos, requests are spread across them.

### Try multiple backends

Run two servers on different ports (separate terminals, from this directory):

```bash
# terminal A — default 9090
cargo run --bin server

# terminal B — override grpc port in a copy of config or env; simplest: second binary with app.toml grpc.port = 9091
```

Register both under the same `service_name` in Nacos (each process registers on start). The client prints all discovered endpoints and issues several `SayHello` calls through the balanced channel.
