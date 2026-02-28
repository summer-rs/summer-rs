+++
title = "Quick Start"
description = "A page introducing how to quickly get started with summer-rs"
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "On this page, I will introduce how to quickly get started with summer-rs"
toc = true
top = false
+++

## Prepare the environment

* rust ≥ 1.75

## Add dependencies

Add the following dependencies to your `Cargo.toml` file

```toml
[dependencies]
# summer provides the core plugin system and useful Procedural Macros
summer = "<version>"
# If you are going to write a web application, add summer-web
summer-web = "<version>"
# If the application needs to interact with the database, add summer-sqlx
summer-sqlx = { version="<version>", features = ["mysql"] }
# The summer-rs project uses the tokio asynchronous runtime by default
tokio = "1"
```

## Write code

```rust
{{ include_code(path="../../examples/hello-world-example/src/main.rs") }}
```

## Configure the application

Create a `config` directory in the root path of the project, where the `summer-rs` configuration files will be stored.

You can first create an `app.toml` file in this directory with the following content:

```toml
[web]
port = 8000 # Configure the web service port. If not configured, the default port is 8080

[sqlx] # Configure the database connection information of sqlx
uri = "mysql://user:password@127.0.0.1:3306"
```

`summer-rs` supports multiple environment configurations: dev (development), test (testing), and prod (production), corresponding to the three configuration files `app-dev.toml`, `app-test.toml`, and `app-prod.toml`. The configuration in the environment configuration file will override the configuration items of the `app.toml` main configuration file.

`summer-rs` will activate the configuration file of the corresponding environment according to the `summer_ENV` environment variable.

## Run

Coding is complete, please make sure your database can be connected normally, then let's start running.

```sh
cargo run
```
