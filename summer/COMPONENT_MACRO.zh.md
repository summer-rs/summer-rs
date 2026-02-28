# Component 宏使用指南

`#[component]` 宏为 spring-rs 应用提供声明式组件注册，无需手动实现 Plugin trait。

## 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
spring = "0.4"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

**注意：** 你不需要直接添加 `spring-macros`、`async-trait` 或 `inventory` 依赖。
`spring` crate 已经为你重新导出了这些依赖。

### 2. 定义组件

```rust
#[derive(Clone)]
struct DbConnection {
    pool: sqlx::PgPool,
}
```

### 3. 创建配置

```rust
use spring::config::Configurable;
use serde::Deserialize;

#[derive(Clone, Configurable, Deserialize)]
#[config_prefix = "database"]
struct DbConfig {
    url: String,
    max_connections: u32,
}
```

### 4. 使用 `#[component]` 宏

```rust
use spring::config::Config;
use spring::component;

#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection {
        pool: sqlx::PgPool::connect(&config.url).await.unwrap(),
    }
}
```

### 5. 在应用中注册

```rust
use spring::App;

#[tokio::main]
async fn main() {
    App::new()
        .run()
        .await;
}
```

## 工作原理

`#[component]` 宏将你的组件创建函数转换为 Plugin 实现：

### 输入

```rust
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}
```

### 生成的代码（概念性）

```rust
struct __CreateDbConnectionPlugin;

#[async_trait]
impl Plugin for __CreateDbConnectionPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // 提取配置
        let config = app.get_config::<DbConfig>()
            .expect("Config DbConfig not found");
        let Config(config) = Config(config);
        
        // 调用原始函数
        let component = create_db_connection(Config(config));
        
        // 注册组件
        app.add_component(component);
    }
    
    fn name(&self) -> &str {
        "__CreateDbConnectionPlugin"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec![]  // 无依赖
    }
}

// 通过 inventory 自动注册
inventory::submit! {
    &__CreateDbConnectionPlugin as &dyn Plugin
}

// 原始函数被保留
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}
```

## 参数类型

### Config<T> - 配置注入

从 `config/app.toml` 注入配置：

```rust
#[component]
fn create_component(
    Config(config): Config<MyConfig>,
) -> MyComponent {
    MyComponent::new(&config)
}
```

**要求：**
- `T` 必须实现 `Configurable + Deserialize`
- 配置必须存在于 `config/app.toml` 中，位于 `#[config_prefix]` 指定的前缀下

### Component<T> - 组件注入

注入另一个组件：

```rust
#[component]
fn create_service(
    Component(db): Component<DbConnection>,
) -> MyService {
    MyService::new(db)
}
```

**要求：**
- `T` 必须是已注册的组件
- 依赖会自动添加到插件的 `dependencies()` 列表中

### 多个参数

你可以混合使用不同的参数类型：

```rust
#[component]
fn create_service(
    Config(config): Config<ServiceConfig>,
    Component(db): Component<DbConnection>,
    Component(cache): Component<RedisClient>,
) -> MyService {
    MyService::new(&config, db, cache)
}
```

## 返回类型

### 简单类型

```rust
#[component]
fn create_component() -> MyComponent {
    MyComponent::new()
}
```

**要求：**
- 必须实现 `Clone + Send + Sync + 'static`

### Result 类型

用于可能失败的初始化：

```rust
#[component]
fn create_component(
    Config(config): Config<MyConfig>,
) -> Result<MyComponent, anyhow::Error> {
    let component = MyComponent::try_new(&config)?;
    Ok(component)
}
```

**注意：** 如果函数返回错误，应用会 panic 并显示错误信息。

### 异步函数

用于异步初始化：

```rust
#[component]
async fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    let pool = sqlx::PgPool::connect(&config.url).await.unwrap();
    DbConnection { pool }
}
```

### 异步 + Result

组合使用异步和 Result：

```rust
#[component]
async fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> Result<DbConnection, sqlx::Error> {
    let pool = sqlx::PgPool::connect(&config.url).await?;
    Ok(DbConnection { pool })
}
```

## 依赖解析

### 自动依赖检测

宏会自动从 `Component<T>` 参数检测依赖：

```rust
#[component]
fn create_repository(
    Component(db): Component<DbConnection>,  // 依赖 DbConnection
) -> UserRepository {
    UserRepository { db }
}
```

生成的 `dependencies()`：
```rust
fn dependencies(&self) -> Vec<&str> {
    vec!["__CreateDbConnectionPlugin"]
}
```

### 初始化顺序

组件按依赖顺序初始化：

```rust
// 1. 无依赖 - 首先初始化
#[component]
fn create_db() -> DbConnection { ... }

// 2. 依赖 DbConnection - 第二个初始化
#[component]
fn create_repo(Component(db): Component<DbConnection>) -> UserRepository { ... }

// 3. 依赖 UserRepository - 第三个初始化
#[component]
fn create_service(Component(repo): Component<UserRepository>) -> UserService { ... }
```

### 循环依赖

**不支持**循环依赖，会导致 panic：

```rust
// ❌ 这会 panic！
#[component]
fn create_a(Component(b): Component<B>) -> A { ... }

#[component]
fn create_b(Component(a): Component<A>) -> B { ... }
```

**解决方案：** 重构设计以消除循环依赖。

## 高级用法

### 自定义插件名称

当需要同一类型的多个组件时使用自定义名称：

```rust
#[derive(Clone)]
struct PrimaryDb(DbConnection);

#[derive(Clone)]
struct SecondaryDb(DbConnection);

#[component(name = "PrimaryDatabase")]
fn create_primary_db(
    Config(config): Config<PrimaryDbConfig>,
) -> PrimaryDb {
    PrimaryDb(DbConnection::new(&config))
}

#[component(name = "SecondaryDatabase")]
fn create_secondary_db(
    Config(config): Config<SecondaryDbConfig>,
) -> SecondaryDb {
    SecondaryDb(DbConnection::new(&config))
}
```

### 显式依赖

使用 `#[inject("PluginName")]` 指定显式依赖：

```rust
#[component]
fn create_repository(
    #[inject("PrimaryDatabase")] Component(db): Component<PrimaryDb>,
) -> UserRepository {
    UserRepository::new(db.0)
}
```

适用场景：
- 依赖有自定义名称
- 想要明确指定依赖哪个插件

### NewType 模式用于多实例

当需要同一类型的多个实例时，使用 NewType 模式：

```rust
#[derive(Clone)]
struct PrimaryCache(RedisClient);

#[derive(Clone)]
struct SecondaryCache(RedisClient);

#[component(name = "PrimaryCache")]
fn create_primary_cache(
    Config(config): Config<PrimaryCacheConfig>,
) -> PrimaryCache {
    PrimaryCache(RedisClient::new(&config))
}

#[component(name = "SecondaryCache")]
fn create_secondary_cache(
    Config(config): Config<SecondaryCacheConfig>,
) -> SecondaryCache {
    SecondaryCache(RedisClient::new(&config))
}

#[component]
fn create_service(
    Component(primary): Component<PrimaryCache>,
    Component(secondary): Component<SecondaryCache>,
) -> CacheService {
    CacheService {
        primary: primary.0,
        secondary: secondary.0,
    }
}
```

### 对大型组件使用 Arc

对于大型组件，使用 `Arc` 减少克隆开销：

```rust
use std::sync::Arc;

#[derive(Clone)]
struct LargeComponent {
    data: Arc<Vec<u8>>,  // 共享数据
}

#[component]
fn create_large_component() -> LargeComponent {
    LargeComponent {
        data: Arc::new(vec![0; 1_000_000]),
    }
}
```

## 最佳实践

### 1. 保持组件函数简单

组件函数应该只创建和配置组件：

```rust
// ✅ 好
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}

// ❌ 不好 - 逻辑太多
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    let conn = DbConnection::new(&config);
    conn.run_migrations();  // 不要在这里做
    conn.seed_data();       // 不要在这里做
    conn
}
```

### 2. 对所有可配置值使用配置

```rust
// ✅ 好
#[component]
fn create_service(
    Config(config): Config<ServiceConfig>,
) -> MyService {
    MyService::new(&config)
}

// ❌ 不好 - 硬编码值
#[component]
fn create_service() -> MyService {
    MyService::new("localhost", 8080)
}
```

### 3. 使用明确的名称提高清晰度

```rust
// ✅ 好 - 意图清晰
#[component(name = "PrimaryDatabase")]
fn create_primary_db(...) -> PrimaryDb { ... }

// ❌ 不够清晰
#[component]
fn create_db1(...) -> Db1 { ... }
```

### 4. 记录组件依赖

```rust
/// 创建 UserService 组件。
///
/// # 依赖
/// - UserRepository: 用于数据访问
/// - RedisClient: 用于缓存
#[component]
fn create_user_service(
    Component(repo): Component<UserRepository>,
    Component(cache): Component<RedisClient>,
) -> UserService {
    UserService::new(repo, cache)
}
```

### 5. 对可能失败的初始化使用 Result

```rust
// ✅ 好 - 显式错误处理
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> Result<DbConnection, anyhow::Error> {
    DbConnection::try_new(&config)
}

// ❌ 不好 - 隐藏的 panic
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::try_new(&config).unwrap()  // 错误时会 panic
}
```

## 故障排查

### 错误："Config X not found"

**原因：** `config/app.toml` 中缺少配置

**解决方案：** 添加配置：

```toml
[your-prefix]
key = "value"
```

### 错误："Component X not found"

**原因：** 依赖的组件未注册

**解决方案：** 确保依赖也使用 `#[component]` 标记，并在此组件之前注册。

### 错误："Cyclic dependency detected"

**原因：** 两个或多个组件相互依赖

**解决方案：** 重构设计以消除循环依赖。考虑：
- 引入中间组件
- 使用事件/回调代替直接依赖
- 重构架构

### 错误："plugin was already added"

**原因：** 两个组件返回相同类型

**解决方案：** 使用 NewType 模式或自定义名称：

```rust
#[derive(Clone)]
struct PrimaryDb(DbConnection);

#[component(name = "PrimaryDatabase")]
fn create_primary_db(...) -> PrimaryDb { ... }
```

### 错误："component was already added"

**原因：** 同一组件类型被注册两次

**解决方案：** 每个组件类型只能注册一次。对多个实例使用 NewType 模式。

## 迁移指南

### 从手动 Plugin 迁移到 `#[component]`

**之前：**

```rust
struct DbConnectionPlugin;

#[async_trait]
impl Plugin for DbConnectionPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app.get_config::<DbConfig>()
            .expect("DbConfig not found");
        
        let db = DbConnection::new(&config);
        app.add_component(db);
    }
    
    fn name(&self) -> &str {
        "DbConnectionPlugin"
    }
}

// 在 main 中
App::new()
    .add_plugin(DbConnectionPlugin)
    .run()
    .await;
```

**之后：**

```rust
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}

// 在 main 中
App::new()
    .run()
    .await;
```

### 迁移步骤

1. **识别组件创建逻辑** 在 Plugin 的 `build` 方法中
2. **提取到函数中** 使用适当的参数
3. **添加 `#[component]` 宏** 到函数上
4. **移除 `add_plugin` 调用** 组件会自动注册
5. **删除手动 Plugin 实现**
6. **测试** 确保一切正常工作

### 兼容性

`#[component]` 宏与手动 Plugin 实现完全兼容。你可以混合使用两种方式：

```rust
App::new()
    .add_plugin(ManualPlugin)  // 手动插件
    .run()
    .await;
```

## 相关资源

- [spring-rs 文档](https://spring-rs.github.io/)
- [插件系统](../spring/Plugin.zh.md)
- [依赖注入](../spring/DI.zh.md)
- [配置系统](../spring/Config.zh.md)
- [示例项目](../examples/component-macro-example/)
