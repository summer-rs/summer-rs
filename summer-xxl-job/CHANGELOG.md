# Changelog

## 0.7.2

- **added**: Service-based lazy handler registration. New APIs
  `add_xxl_async_service::<H>(name)` and `add_xxl_sync_service::<H>(name)`
  allow registering handlers that derive `Service` and use
  `#[inject(component)]` / `#[inject(config)]` fields. Handlers are
  instantiated by the plugin after Summer's DI phase completes via a
  `ServicesInjectedEvent` listener.
- **added**: lower-level escape hatch `add_xxl_lazy_handler(name, factory)`
  along with `LazyJobFactory` / `XxlLazyHandlerRegistry` types for custom
  deferred-construction scenarios.

## 0.6.0

- Initial release. Integrate `xxljob-sdk-rs` as a Summer plugin so that an
  application can act as an executor for xxl-job-admin / ratch-job.
