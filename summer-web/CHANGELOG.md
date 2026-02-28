# Changelog

## 0.5.0

- **changed**: upgrade `summer` 0.4 to 0.5 ([#217])

[#217]: https://github.com/summer-rs/summer-rs/pull/217

## 0.4.13

- **changed**: upgrade `schemars` 0.9 to 1.1 ([#197])

[#197]: https://github.com/summer-rs/summer-rs/pull/197

## 0.4.12

- **added**: AppRef impl OperationInput ([#194])

[#194]: https://github.com/summer-rs/summer-rs/pull/194

## 0.4.11

- **added**: AppRef extractor ([#190])

[#190]: https://github.com/summer-rs/summer-rs/pull/190

## 0.4.10

- **added**: Socket io support using Socketoxide ([#176])

[#176]: https://github.com/summer-rs/summer-rs/pull/176

## 0.4.9

- **added**: refactor `shutdown_signal` ([#180])

[#180]: https://github.com/summer-rs/summer-rs/pull/180

## 0.4.8

- feat: ✨ openapi feature ([#167])

[#167]: https://github.com/summer-rs/summer-rs/pull/167

## 0.4.7

- feat: ✨ Enhance middleware functionality with route-specific middleware support ([#157])

[#157]: https://github.com/summer-rs/summer-rs/pull/157

## 0.4.6

- feat: ✨ summer macros support debug_handler ([#156])

[#156]: https://github.com/summer-rs/summer-rs/pull/156

## 0.4.5

- **added**: serde derive

## 0.4.4

- **added**: feat: ✨ add middlewares macro ([#139])

[#139]: https://github.com/summer-rs/summer-rs/pull/139

## 0.4.3

- **breaking**: upgrade `summer` and `summer-macros` 0.3 to 0.4 ([#132])

[#132]: https://github.com/summer-rs/summer-rs/pull/132

## 0.4.2

- **added**: add static_dir for fallback (#ccf3dd)

[#ccf3dd]: https://github.com/summer-rs/summer-rs/commit/ccf3dd139cd9e67854940343163f027457ac2dc8

## 0.4.1

- **added**: fix *Nesting at the root is no longer supported. Use fallback_service instead* (#56805b)

[#56805b]: https://github.com/summer-rs/summer-rs/commit/56805baea3de500287d0ef447ff48c28b095e4ba

## 0.4.0

- **breaking**: upgrade axum to 0.8 ([#122])

[#122]: https://github.com/summer-rs/summer-rs/pull/122

## 0.3.0

- **breaking**: refactor dependency inject ([#105])

[#105]: https://github.com/summer-rs/summer-rs/pull/105

## 0.2.4

- **changed**: fix summer-web cors middleware support wildcard ([#102])

[#102]: https://github.com/summer-rs/summer-rs/pull/102

## 0.2.4

- **changed**: upgrade `tower-http = "0.5"` to `tower-http = "0.6"` ([#76])

[#76]: https://github.com/summer-rs/summer-rs/pull/76

## 0.2.3

- **changed**: http2 as the default feature ([#65])

[#65]: https://github.com/summer-rs/summer-rs/pull/65

## 0.2.2

- **changed**: refactor KnownWebError: change &str to Into<String> ([#6f243a])

[#6f243a]: https://github.com/summer-rs/summer-rs/commit/6f243aa384aee22a0f3a32ed2ea2f20ec0f4d513

## 0.2.1

- **added**: support graceful shutdown ([#61])

[#61]: https://github.com/summer-rs/summer-rs/pull/61

## 0.2.0

- **added**: support dependency inject ([#54])
- **changed**: rename TypedHandlerFactory to TypedHandlerRegistrar ([#54])

[#54]: https://github.com/summer-rs/summer-rs/pull/54

## 0.1.5

- **changed**: update debug log ([#52])

[#52]: https://github.com/summer-rs/summer-rs/pull/52

## 0.1.4

- **added**: support ConnectInfo ([#51])

[#51]: https://github.com/summer-rs/summer-rs/pull/51

## 0.1.3

- **added**: Support extracting Component in middleware ([#44])
- **changed**: web-middleware-example support problemdetail ([#44])

[#44]: https://github.com/summer-rs/summer-rs/pull/44

## 0.1.2

- **added**: WebError IntoResponse add log ([#5c684e])

[#5c684e]: https://github.com/summer-rs/summer-rs/commit/5c684e439f4a8877aebcbb091bdc404bdf982597

## before 0.1.1

see [CHANGELOG](../CHANGELOG.md)