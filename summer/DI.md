## Compile-time dependency inject

spring-rs provides a special Component - [Service](https://docs.rs/spring/latest/spring/plugin/service/index.html), which supports injecting dependent components at compile time.

Like the following example, `UserService` only needs to derive the `Service` trait. In order to distinguish the injected dependencies, you need to specify whether the dependency is a Component or a Config through the attribute macros `#[inject(component)]` and `#[inject(config)]`.

```rust
use spring_sqlx::ConnectPool;
use spring::config::Configurable;
use spring::plugin::service::Service;
use serde::Deserialize;

#[derive(Clone, Configurable, Deserialize)]
#[config_prefix = "user"]
struct UserConfig {
    username: String,
    project: String,
}

#[derive(Clone, Service)]
struct UserService {
    #[inject(component)]
    db: ConnectPool,
    #[inject(config)]
    config: UserConfig,
}

#[derive(Clone, Service)]
struct UserWithOptionalComponentService {
    #[inject(component)]
    db: Option<ConnectPool>, // If ConnectPool does not exist, inject None
    #[inject(config)]
    config: UserConfig,
}
```

For the complete code, see [`dependency-inject-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/dependency-inject-example).

> Service also supports grpc mode and can be used in conjunction with the [spring-grpc](https://spring-rs.github.io/docs/plugins/spring-grpc/) plug-in

## Nested dependency inject

spring-rs supports multi-level dependency injection. For example, if `UserService` depends on `OtherService`, and `OtherService` depends on `DatabaseService`, then when you inject `UserService`, `OtherService` and `DatabaseService` will be automatically injected.

```rust
use spring::plugin::LazyComponent;
use spring::plugin::service::Service;

#[derive(Clone, Service)]
struct DatabaseService {
    // ...
}
#[derive(Clone, Service)]
struct OtherService {
    #[inject(component)]
    db: DatabaseService,
}
#[derive(Clone, Service)]
struct UserService {
    #[inject(component)]
    other_service: OtherService,
}
```

For the complete code, see [`nested-dependency-inject-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/nested-dependency-inject-example).

## Circular dependency inject

When two services reference each other, Rust's type system prevents direct circular dependencies. To solve this, you can use `LazyComponent<T>` to break the circular dependency.

> The dependency injection philosophy of spring-rs is inspired by [Google’s Dagger](https://github.com/google/dagger). Circular dependencies are discouraged, as they usually imply unclear business responsibilities and tight coupling.

```rust
use spring::plugin::LazyComponent;
use spring::plugin::service::Service;

#[derive(Clone, Service)]
struct UserService {
    #[inject(component)] // It's optional to use #[inject] in this case
    other_service: LazyComponent<OtherService>,  // ✅ Lazy resolution
}

#[derive(Clone, Service)]
struct OtherService {
    #[inject(component)]
    user_service: UserService,  // ✅ Direct injection OK
    // And you can also make this lazy if needed
    // user_service: LazyComponent<UserService>,
}
```

This allows `UserService` to hold a lightweight reference to `OtherService` that is only resolved when needed. To access the actual component, call `.get()`.

Both sides of the circular dependency can be lazy if needed, but only one side needs to be lazy, so choose the less frequently accessed service to be lazy.

It's not necessary to use the `#[inject]` attribute with `LazyComponent<T>`, as it is automatically detected. 
Internally, this type is just a wrapper around `Arc<RwLock<...>>`, making it thread-safe.


For the complete code, see [`circular-dependency-injection-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/circular-dependency-injection-example)

