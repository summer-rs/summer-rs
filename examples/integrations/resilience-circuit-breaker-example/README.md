# FX quote circuit breaker example

This example runs two Summer applications. The FX provider returns two real HTTP `503 Service
Unavailable` responses, then becomes healthy. The client uses `#[circuit_breaker]`: two failed HTTP
requests open the circuit, the next call is rejected without network traffic, and a later half-open
HTTP probe succeeds and closes the circuit.

Run both commands from this directory so each application loads `config/app.toml`.

Start the provider:

```bash
cargo run --bin fx-provider
```

Then run the client in another terminal:

```bash
cargo run --bin circuit-breaker-client
```
