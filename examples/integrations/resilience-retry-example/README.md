# FX quote retry example

This example runs two Summer applications. The FX provider serves a real HTTP endpoint that returns
`503 Service Unavailable` twice before returning a USD/THB JSON quote. The client makes a normal
HTTP request through `#[retry]`, which performs three attempts with exponential backoff.

Run both commands from this directory so each application loads `config/app.toml`.

Start the provider:

```bash
cargo run --bin fx-provider
```

Then run the client in another terminal:

```bash
cargo run --bin retry-client
```
