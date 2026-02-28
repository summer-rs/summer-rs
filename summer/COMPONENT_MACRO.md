# Component Macro Guide

The `#[component]` macro provides declarative component registration for spring-rs applications, eliminating the need to manually implement the Plugin trait.

## Quick Start

### 1. Add Dependencies

Add `spring` and `spring-macros` to your `Cargo.toml`:

```toml
[dependencies]
spring = "0.4"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

**Note:** You don't need to add `spring-macros`, `async-trait` or `inventory` as direct dependencies.
The `spring` crate re-exports these for you.

### 2. Define Your Component

```rust
#[derive(Clone)]
struct DbConnection {
    pool: sqlx::PgPool,
}
```

### 2. Create Configuration

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

### 3. Use `#[component]` Macro

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

### 4. Register in Application

```rust
use spring::App;

#[tokio::main]
async fn main() {
    App::new()
        .run()
        .await;
}
```

## How It Works

The `#[component]` macro transforms your component creation function into a Plugin implementation:

### Input

```rust
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}
```

### Generated Code (Conceptual)

```rust
struct __CreateDbConnectionPlugin;

#[async_trait]
impl Plugin for __CreateDbConnectionPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        // Extract configuration
        let config = app.get_config::<DbConfig>()
            .expect("Config DbConfig not found");
        let Config(config) = Config(config);
        
        // Call original function
        let component = create_db_connection(Config(config));
        
        // Register component
        app.add_component(component);
    }
    
    fn name(&self) -> &str {
        "__CreateDbConnectionPlugin"
    }
    
    fn dependencies(&self) -> Vec<&str> {
        vec![]  // No dependencies
    }
}

// Auto-register via inventory
inventory::submit! {
    &__CreateDbConnectionPlugin as &dyn Plugin
}

// Original function is preserved
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}
```

## Parameter Types

### Config<T> - Configuration Injection

Injects configuration from `config/app.toml`:

```rust
#[component]
fn create_component(
    Config(config): Config<MyConfig>,
) -> MyComponent {
    MyComponent::new(&config)
}
```

**Requirements:**
- `T` must implement `Configurable + Deserialize`
- Configuration must exist in `config/app.toml` under the prefix specified by `#[config_prefix]`

### Component<T> - Component Injection

Injects another component:

```rust
#[component]
fn create_service(
    Component(db): Component<DbConnection>,
) -> MyService {
    MyService::new(db)
}
```

**Requirements:**
- `T` must be a registered component
- The dependency will be automatically added to the plugin's `dependencies()` list

### Multiple Parameters

You can mix and match parameter types:

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

## Return Types

### Simple Type

```rust
#[component]
fn create_component() -> MyComponent {
    MyComponent::new()
}
```

**Requirements:**
- Must implement `Clone + Send + Sync + 'static`

### Result Type

For fallible initialization:

```rust
#[component]
fn create_component(
    Config(config): Config<MyConfig>,
) -> Result<MyComponent, anyhow::Error> {
    let component = MyComponent::try_new(&config)?;
    Ok(component)
}
```

**Note:** If the function returns an error, the application will panic with the error message.

### Async Functions

For async initialization:

```rust
#[component]
async fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    let pool = sqlx::PgPool::connect(&config.url).await.unwrap();
    DbConnection { pool }
}
```

### Async + Result

Combine async and Result:

```rust
#[component]
async fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> Result<DbConnection, sqlx::Error> {
    let pool = sqlx::PgPool::connect(&config.url).await?;
    Ok(DbConnection { pool })
}
```

## Dependency Resolution

### Automatic Dependency Detection

The macro automatically detects dependencies from `Component<T>` parameters:

```rust
#[component]
fn create_repository(
    Component(db): Component<DbConnection>,  // Depends on DbConnection
) -> UserRepository {
    UserRepository { db }
}
```

Generated `dependencies()`:
```rust
fn dependencies(&self) -> Vec<&str> {
    vec!["__CreateDbConnectionPlugin"]
}
```

### Initialization Order

Components are initialized in dependency order:

```rust
// 1. No dependencies - initialized first
#[component]
fn create_db() -> DbConnection { ... }

// 2. Depends on DbConnection - initialized second
#[component]
fn create_repo(Component(db): Component<DbConnection>) -> UserRepository { ... }

// 3. Depends on UserRepository - initialized third
#[component]
fn create_service(Component(repo): Component<UserRepository>) -> UserService { ... }
```

### Circular Dependencies

Circular dependencies are **not supported** and will cause a panic:

```rust
// ❌ This will panic!
#[component]
fn create_a(Component(b): Component<B>) -> A { ... }

#[component]
fn create_b(Component(a): Component<A>) -> B { ... }
```

**Solution:** Refactor your design to eliminate the circular dependency.

## Advanced Usage

### Custom Plugin Names

Use custom names when you need multiple components of the same type:

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

### Explicit Dependencies

Use `#[inject("PluginName")]` to specify explicit dependencies:

```rust
#[component]
fn create_repository(
    #[inject("PrimaryDatabase")] Component(db): Component<PrimaryDb>,
) -> UserRepository {
    UserRepository::new(db.0)
}
```

This is useful when:
- The dependency has a custom name
- You want to be explicit about which plugin to depend on

### NewType Pattern for Multiple Instances

When you need multiple instances of the same type, use the NewType pattern:

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

### Using Arc for Large Components

For large components, use `Arc` to reduce clone overhead:

```rust
use std::sync::Arc;

#[derive(Clone)]
struct LargeComponent {
    data: Arc<Vec<u8>>,  // Shared data
}

#[component]
fn create_large_component() -> LargeComponent {
    LargeComponent {
        data: Arc::new(vec![0; 1_000_000]),
    }
}
```

## Best Practices

### 1. Keep Component Functions Simple

Component functions should only create and configure the component:

```rust
// ✅ Good
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}

// ❌ Bad - too much logic
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    let conn = DbConnection::new(&config);
    conn.run_migrations();  // Don't do this here
    conn.seed_data();       // Don't do this here
    conn
}
```

### 2. Use Configuration for All Configurable Values

```rust
// ✅ Good
#[component]
fn create_service(
    Config(config): Config<ServiceConfig>,
) -> MyService {
    MyService::new(&config)
}

// ❌ Bad - hardcoded values
#[component]
fn create_service() -> MyService {
    MyService::new("localhost", 8080)
}
```

### 3. Prefer Explicit Names for Clarity

```rust
// ✅ Good - clear intent
#[component(name = "PrimaryDatabase")]
fn create_primary_db(...) -> PrimaryDb { ... }

// ❌ Less clear
#[component]
fn create_db1(...) -> Db1 { ... }
```

### 4. Document Component Dependencies

```rust
/// Creates the UserService component.
///
/// # Dependencies
/// - UserRepository: For data access
/// - RedisClient: For caching
#[component]
fn create_user_service(
    Component(repo): Component<UserRepository>,
    Component(cache): Component<RedisClient>,
) -> UserService {
    UserService::new(repo, cache)
}
```

### 5. Use Result for Fallible Initialization

```rust
// ✅ Good - explicit error handling
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> Result<DbConnection, anyhow::Error> {
    DbConnection::try_new(&config)
}

// ❌ Bad - hidden panic
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::try_new(&config).unwrap()  // Will panic on error
}
```

## Troubleshooting

### Error: "Config X not found"

**Cause:** Configuration is missing from `config/app.toml`

**Solution:** Add the configuration:

```toml
[your-prefix]
key = "value"
```

### Error: "Component X not found"

**Cause:** The dependency component is not registered

**Solution:** Ensure the dependency is also marked with `#[component]` and registered before this component.

### Error: "Cyclic dependency detected"

**Cause:** Two or more components depend on each other

**Solution:** Refactor your design to eliminate the circular dependency. Consider:
- Introducing an intermediate component
- Using events/callbacks instead of direct dependencies
- Restructuring your architecture

### Error: "plugin was already added"

**Cause:** Two components return the same type

**Solution:** Use the NewType pattern or custom names:

```rust
#[derive(Clone)]
struct PrimaryDb(DbConnection);

#[component(name = "PrimaryDatabase")]
fn create_primary_db(...) -> PrimaryDb { ... }
```

### Error: "component was already added"

**Cause:** The same component type is registered twice

**Solution:** Each component type can only be registered once. Use NewType pattern for multiple instances.

## Migration Guide

### From Manual Plugin to `#[component]`

**Before:**

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

// In main
App::new()
    .add_plugin(DbConnectionPlugin)
    .run()
    .await;
```

**After:**

```rust
#[component]
fn create_db_connection(
    Config(config): Config<DbConfig>,
) -> DbConnection {
    DbConnection::new(&config)
}

// In main
App::new()
    .run()
    .await;
```

### Migration Steps

1. **Identify component creation logic** in your Plugin's `build` method
2. **Extract it into a function** with appropriate parameters
3. **Add `#[component]` macro** to the function
4. **Replace `add_plugin`** with `add_auto_plugins` in your main function
5. **Remove the manual Plugin implementation**
6. **Test** to ensure everything works

### Compatibility

The `#[component]` macro is fully compatible with manual Plugin implementations. You can mix both approaches:

```rust
App::new()
    .add_plugin(ManualPlugin)  // Manual plugin
    .run()
    .await;
```

## See Also

- [spring-rs Documentation](https://spring-rs.github.io/)
- [Plugin System](../spring/Plugin.md)
- [Dependency Injection](../spring/DI.md)
- [Configuration](../spring/Config.md)
- [Example Project](../examples/component-macro-example/)
