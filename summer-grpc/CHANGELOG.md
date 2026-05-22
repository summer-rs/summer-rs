# Changelog

## 0.7.0

- **added**: after the gRPC TCP listener binds, publish `summer::event::ServerStartedEvent` on the app `EventBus` with `ServerProtocol::Grpc`, so plugins (e.g. Nacos naming) can react before `serve` runs.
- **changed**: bump to **0.7.0** and align the `summer` path dependency `version` pin to **0.7.0**.

## 0.5.0

- **changed**: upgrade `summer` 0.4 to 0.5 ([#217])

[#217]: https://github.com/summer-rs/summer-rs/pull/217

## 0.4.3

- **changed**: upgrade `schemars` 0.9 to 1.1 ([#197])

[#197]: https://github.com/summer-rs/summer-rs/pull/197

## 0.4.2

- **added**: serde derive
- **added**: export `summer::submit_config_schema`

## 0.4.1

- **added**: serde derive

## 0.4.0

- **added**: support grpc in tonic ([#132])

[#132]: https://github.com/summer-rs/summer-rs/pull/132
