## 编译期依赖注入

spring-rs提供了一种特殊的Component——[Service](https://docs.rs/spring/latest/spring/plugin/service/index.html)，它支持在编译期注入依赖的组件。

像下面的例子`UserService`只需派生`Service`特征，为了区分注入的依赖，你需要通过属性宏`#[inject(component)]`和`#[inject(config)]`指定依赖是一个Component还是一个Config。

```rust
use spring_sqlx::ConnectPool;
use spring::config::Configurable;
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
    db: Option<ConnectPool>, // 如果ConnectPool不存在，则输入None
    #[inject(config)]
    config: UserConfig,
}
```

完整代码参考[`dependency-inject-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/dependency-inject-example)。

> Service还支持grpc模式，可结合[spring-grpc](https://spring-rs.github.io/zh/docs/plugins/spring-grpc/)插件一起使用

## 嵌套依赖注入（Nested dependency inject）

spring-rs 支持多层级的依赖注入。
例如，如果 `UserService` 依赖于 `OtherService`，而 `OtherService` 又依赖于 `DatabaseService`，
那么当你注入 `UserService` 时，`OtherService` 和 `DatabaseService` 也会被自动注入。

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

完整代码请参见 [`nested-dependency-inject-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/nested-dependency-inject-example)。

---

## 循环依赖注入（Circular dependency inject）

当两个服务互相引用时，Rust 的类型系统会阻止直接的循环依赖。
为了解决这个问题，你可以使用 `LazyComponent<T>` 来打破循环依赖。

> spring-rs的依赖注入设计理念参照[google的Dagger](https://github.com/google/dagger)，我们不鼓励循环依赖，循环依赖往往意味着：业务职责不清、高耦合。

```rust
use spring::plugin::LazyComponent;
use spring::plugin::service::Service;

use spring::plugin::LazyComponent;
#[derive(Clone, Service)]
struct UserService {
    #[inject(component)] // 在此情况下可选
    other_service: LazyComponent<OtherService>,  // ✅ 延迟解析
}

#[derive(Clone, Service)]
struct OtherService {
    #[inject(component)]
    user_service: UserService,  // ✅ 可直接注入
    // 如果需要，也可以使用延迟注入
    // user_service: LazyComponent<UserService>,
}
```

这样，`UserService` 就可以持有一个指向 `OtherService` 的轻量级引用，
只有在真正需要时才会进行解析。
若要访问实际的组件，请调用 `.get()` 方法。

如果需要，循环依赖的两方都可以设置为延迟注入，但只需一方使用延迟注入即可。
建议将访问频率较低的服务设为延迟注入。

使用 `LazyComponent<T>` 时不必显式添加 `#[inject]` 属性，框架会自动检测。
在内部，它只是对 `Arc<RwLock<...>>` 的封装，因此是线程安全的。

完整代码请参见 [`circular-dependency-injection-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/circular-dependency-injection-example)。
