//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io)
//!
//! Testing utilities for the [summer-rs](https://docs.rs/summer) framework.
//!
//! Currently this crate provides a Web E2E harness built on top of
//! [`axum_test`](https://docs.rs/axum-test):
//!
//! * [`MockWebPlugin`] replaces [`summer_web::WebPlugin`] in tests. It reuses
//!   the production router-assembly path ([`summer_web::assemble_router`] /
//!   [`summer_web::finalize_router`]) so handlers, middlewares, layers and
//!   OpenAPI behave identically to runtime — but the resulting router is
//!   wrapped in an in-memory [`axum_test::TestServer`] instead of binding a TCP
//!   listener, and **no scheduler is registered** so `App::new().build()` returns
//!   immediately.
//! * The [`MockServer`] handle is stored as a normal component on the built
//!   [`summer::App`] and dereferences to [`axum_test::TestServer`], enabling a
//!   one-liner E2E flow:
//!
//! ```no_run
//! # use summer::app::App;
//! # use summer::plugin::ComponentRegistry;
//! # use summer_web::{Router, WebConfigurator};
//! # use summer_test::{MockWebPlugin, MockServer};
//! # async fn run() -> summer::error::Result<()> {
//! App::new()
//!     .add_plugin(MockWebPlugin)
//!     .add_router(Router::new())
//!     .build()
//!     .await?
//!     .get_expect_component::<MockServer>()
//!     .get("/ping")
//!     .await
//!     .assert_status_ok();
//! # Ok(())
//! # }
//! ```
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

#[cfg(feature = "web")]
mod web;

#[cfg(feature = "web")]
pub use web::{MockServer, MockWebPlugin};

/// Re-export of [`axum_test`] for convenience in test code.
#[cfg(feature = "web")]
pub use axum_test;
