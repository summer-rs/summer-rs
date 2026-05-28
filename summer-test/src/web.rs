//! Web E2E testing harness.
//!
//! Provides [`MockWebPlugin`] (a drop-in replacement for [`summer_web::WebPlugin`])
//! and [`MockServer`] (an [`axum_test::TestServer`] handle stored as a normal
//! component on the built [`summer::App`]).

use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use axum_test::TestServer;
use summer::app::AppBuilder;
use summer::async_trait;
use summer::event::{
    AppBuiltEvent, AppEventSubscriber, EventPublisher, ServerProtocol, ServerStartedEvent,
};
use summer::plugin::{MutableComponentRegistry, Plugin};

/// In-memory test server handle published as a component on the built [`summer::App`].
///
/// Internally wraps `Arc<OnceLock<axum_test::TestServer>>`:
/// * The plugin registers an empty `MockServer` during the [`AppBuilder`] phase
///   (so it is visible via [`ComponentRegistry::get_expect_component`] right
///   after [`AppBuilder::build`]).
/// * The actual [`TestServer`] is built inside the [`AppBuiltEvent`] listener
///   (when an `Arc<App>` is finally available for [`summer_web::finalize_router`])
///   and stored into the same `OnceLock`, so all clones observe it.
///
/// Implements [`Deref<Target = TestServer>`] so test code can chain calls
/// directly: `app.get_expect_component::<MockServer>().get("/ping").await`.
pub struct MockServer {
    inner: Arc<OnceLock<TestServer>>,
}

impl MockServer {
    /// Construct an empty handle waiting for [`MockServer::fill`].
    fn pending() -> Self {
        Self {
            inner: Arc::new(OnceLock::new()),
        }
    }

    /// Install the real [`TestServer`]; subsequent calls are no-ops.
    fn fill(&self, server: TestServer) {
        let _ = self.inner.set(server);
    }
}

impl Clone for MockServer {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Deref for MockServer {
    type Target = TestServer;

    fn deref(&self) -> &Self::Target {
        self.inner.get().expect(
            "MockServer is not initialized yet; ensure AppBuilder::build() has completed before \
             dereferencing it",
        )
    }
}

/// Drop-in replacement for [`summer_web::WebPlugin`] in tests.
///
/// Behaviour:
/// 1. During [`Plugin::build`], reuses [`summer_web::assemble_router`] to
///    perform the same router merge / middleware / Socket.IO / OpenAPI setup
///    as production.
/// 2. Registers a pending [`MockServer`] component immediately so it can be
///    retrieved right after [`AppBuilder::build`].
/// 3. Subscribes to [`AppBuiltEvent`]: once the `Arc<App>` is available, calls
///    [`summer_web::finalize_router`] (router layers, OpenAPI finish,
///    [`summer_web::AppState`] extension, global prefix nesting) and wraps the
///    resulting [`axum::Router`] in [`axum_test::TestServer`], filling the
///    [`MockServer`] handle.
/// 4. Publishes a synthetic [`ServerStartedEvent`] (addr `127.0.0.1:0`,
///    protocol `Http`) so listeners (e.g. `summer-nacos`) keep their
///    invariants. **No scheduler is registered**, hence
///    [`AppBuilder::build`] returns immediately without entering a serve loop.
///
/// `name()` returns `"summer_web::WebPlugin"` so other plugins that depend on
/// the production web plugin name (via [`Plugin::dependencies`]) continue to
/// resolve correctly when this mock is used in tests.
pub struct MockWebPlugin;

#[async_trait]
impl Plugin for MockWebPlugin {
    fn name(&self) -> &str {
        // Re-use the production plugin name so dependencies("summer_web::WebPlugin")
        // declared by other plugins keep working under tests.
        "summer_web::WebPlugin"
    }

    async fn build(&self, app: &mut AppBuilder) {
        // 1. Same router-assembly pass as WebPlugin (Routers merge, middlewares,
        //    optional Socket.IO, Router/OpenApiConfig components).
        let server_conf = summer_web::assemble_router(app).await;
        let global_prefix = server_conf.global_prefix.clone();

        // 2. Register the pending handle now, so test code can retrieve it
        //    via `app.get_expect_component::<MockServer>()` right after
        //    `AppBuilder::build` resolves.
        let mock_server = MockServer::pending();
        app.add_component(mock_server.clone());

        // 3. Finalize the router once we actually have an `Arc<App>`
        //    (AppBuiltEvent fires inside `AppBuilder::build_app`, after
        //    `build_plugins` + service injection completed and components
        //    moved into the App).
        app.listen_app(move |event: AppBuiltEvent, _app: &summer::app::App| {
            let global_prefix = global_prefix.clone();
            let mock_server = mock_server.clone();
            let app_arc = event.app.clone();
            async move {
                let router = summer_web::finalize_router(&app_arc, &global_prefix);
                // `TestServer::new` panics if the underlying `IntoTransportLayer`
                // setup fails; an in-memory router cannot fail here, so this is
                // acceptable for tests.
                let server = TestServer::new(router);
                mock_server.fill(server);

                // Synthetic event so summer-nacos / other listeners keep their
                // invariants under test.
                app_arc
                    .publish(ServerStartedEvent {
                        addr: "127.0.0.1:0".parse().expect("static SocketAddr"),
                        protocol: ServerProtocol::Http,
                    })
                    .await?;

                Ok::<(), summer::error::AppError>(())
            }
        });
    }
}
