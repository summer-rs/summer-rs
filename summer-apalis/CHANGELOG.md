# Changelog

## 0.7.2

- **changed**: upgrade `apalis` 1.0.0-rc.8 to **1.0.0** (stable) and adapt to the new registry API. ([#254])
- **changed**: `apalis-postgres` / `apalis-sqlite` features now enable `tokio-comp` / `migrate` (and `json` for SQLite) on the corresponding backends.
- **breaking**: temporarily disable the `sql-mysql` feature: `apalis-mysql` remains at 1.0.0-rc.8 and is incompatible with apalis 1.0.0.

[#254]: https://github.com/summer-rs/summer-rs/pull/254

## 0.7.0

- **changed**: bump to **0.7.0** (stable, aligned with the workspace; supersedes **0.6.0-rc.1**) and align path dependency `version` pins on `summer` / `summer-redis` / `summer-sqlx` to **0.7.0**.

## 0.5.0

- **changed**: upgrade `summer` 0.4 to 0.5 ([#217])

[#217]: https://github.com/summer-rs/summer-rs/pull/217

## 0.4.2

- **changed**: upgrade `apalis` 0.7 to 1.0.0 ([#199])

[#199]: https://github.com/summer-rs/summer-rs/pull/199

## 0.4.1

- **changed**: upgrade `schemars` 0.9 to 1.1 ([#197])

[#197]: https://github.com/summer-rs/summer-rs/pull/197