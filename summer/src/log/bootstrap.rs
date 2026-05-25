//! Early-stage logging bootstrap.
//!
//! On `App::new()` we install a *live* tracing subscriber composed of two
//! [`reload`] slots:
//!
//! 1. **Layer slot** — a `Vec<BoxLayer>` initially holding a single
//!    placeholder stderr `fmt::Layer`. [`LogPlugin::immediately_build`]
//!    swaps in the user-configured layer stack (file appender, stdout
//!    fmt with chrono timer, [`ErrorLayer`], …).
//! 2. **Filter slot** — an [`EnvFilter`] initialized from `RUST_LOG` or
//!    falling back to `info`. The plugin swaps in the directive built
//!    from `LoggerConfig`.
//!
//! The placeholder subscriber is registered as the global default
//! immediately, and [`tracing_log::LogTracer`] is wired up next, so any
//! `log::*` record produced during the gap between `App::new()` and the
//! plugin build phase (e.g. the first `log::warn!("nacos config not
//! found")` from a config-loading plugin) reaches a real subscriber
//! instead of being dropped by `NoSubscriber`.
//!
//! Both globals (`tracing::dispatcher::set_global_default` and
//! `log::set_boxed_logger`) are touched exactly once per process; the
//! later swap is a `Handle::modify` against the in-place reload layers,
//! so it never races with the one-shot global setters.
//!
//! [`LogPlugin::immediately_build`]: super::LogPlugin
//! [`ErrorLayer`]: tracing_error::ErrorLayer

use std::sync::{Once, OnceLock};

use tracing_log::LogTracer;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::{Layered, SubscriberExt};
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

/// Boxed [`Layer`] used as the leaf element of the layer-reload slot's
/// `Vec`. Re-exported by [`super`] for users assembling their own layers.
pub type BoxLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Subscriber type after the inner (layer) reload slot has been wrapped on
/// top of [`Registry`]. Used as the `S` parameter of the outer (filter)
/// reload slot so that [`EnvFilter`]'s `Layer<S>` impl picks the right
/// subscriber to operate on.
type LayerSubscriber = Layered<reload::Layer<Vec<BoxLayer>, Registry>, Registry>;

/// Reload handle for the layer slot. `LogPlugin` `modify`s this with the
/// user's `Vec<BoxLayer>`.
pub(super) static LAYER_RELOAD_HANDLE: OnceLock<reload::Handle<Vec<BoxLayer>, Registry>> =
    OnceLock::new();

/// Reload handle for the filter slot. `LogPlugin` `modify`s this with the
/// `EnvFilter` built from `LoggerConfig`.
pub(super) static FILTER_RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, LayerSubscriber>> =
    OnceLock::new();

static INSTALL: Once = Once::new();

/// Install the early bootstrap subscriber and `log -> tracing` bridge.
///
/// Idempotent. Safe to call from `AppBuilder::default()`.
pub fn install_bootstrap_logger() {
    INSTALL.call_once(|| {
        // Placeholder leaf layer: stderr, no time formatting. Replaced
        // wholesale by `LogPlugin::immediately_build`.
        let initial_layer: BoxLayer = fmt::layer().with_writer(std::io::stderr).boxed();
        let (layer_slot, layer_handle) = reload::Layer::new(vec![initial_layer]);

        // Default to `RUST_LOG` if set, otherwise `info`. The plugin
        // build step will replace this with whatever the user-configured
        // `LoggerConfig` resolves to.
        let initial_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let (filter_slot, filter_handle) = reload::Layer::new(initial_filter);

        let _ = LAYER_RELOAD_HANDLE.set(layer_handle);
        let _ = FILTER_RELOAD_HANDLE.set(filter_handle);

        // Layer order matters: `layer_slot` (Layer<Registry>) sits inner,
        // `filter_slot` (EnvFilter, which is Layer<S> for any S) sits
        // outer so that filter callsite-interest decisions act as a
        // global veto in front of the layer stack.
        let _ = tracing_subscriber::registry()
            .with(layer_slot)
            .with(filter_slot)
            .try_init();

        // Once the dispatcher is in place, bridge `log::*` records to it.
        let _ = LogTracer::init();
    });
}
