[![crates.io](https://img.shields.io/crates/v/spring-sa-token.svg)](https://crates.io/crates/spring-sa-token)
[![Documentation](https://docs.rs/spring-sa-token/badge.svg)](https://docs.rs/spring-sa-token)

`spring-sa-token` 是针对[sa-token-rust](https://github.com/click33/sa-token-rust)的自动装配。sa-token-rust 。


## 依赖

```toml
# 默认：内存存储（用于开发）
spring-sa-token = { version = "<version>" }
# 生产环境：复用 spring-redis 连接（推荐）
spring-sa-token = { version = "<version>", default-features = false, features = ["with-spring-redis", "with-web"] }
```

可选 **features**：
* `memory`：内存存储（默认，用于开发/测试）
* `with-spring-redis`：使用 spring-redis 连接池存储（推荐）
* `with-web`：启用 axum web 集成（中间件、提取器）

## 配置项
具体文档、配置可以查看 [sa-token-rust docs](https://github.com/click33/sa-token-rust/tree/main/docs)

```toml
[sa-token]
# Token 名称（header 或 cookie 中的键名）
token_name = "Authorization"

# Token 有效期（秒），-1 表示永久有效
# 默认：2592000（30 天）
timeout = 86400

# Token 最低活跃频率（秒），-1 表示不限制
# 如果在此时间内没有请求，Token 将失效
active_timeout = 3600

# 是否开启自动续签 - 每次请求自动刷新 Token
auto_renew = true

# 是否允许同一账号并发登录
is_concurrent = true

# 在多人登录同一账号时，是否共享一个 Token
is_share = true

# Token 风格：Uuid, SimpleUuid, Random32, Random64, Random128, Jwt
token_style = "Uuid"

# Token 前缀（如 "Bearer "）
token_prefix = "Bearer "

# JWT 配置（仅当 token_style = "Jwt" 时需要）
jwt_secret_key = "your-secret-key"
jwt_algorithm = "HS256"    # HS256, HS384, HS512
jwt_issuer = "my-app"
jwt_audience = "my-users"

# 是否启用防重放攻击（nonce 机制）
enable_nonce = false
nonce_timeout = 300

# 是否启用 Refresh Token
enable_refresh_token = false
refresh_token_timeout = 604800  # 7 天
```


## 快速开始

### 1. 添加插件到应用

```rust
use spring::{auto_config, App};
use spring_redis::RedisPlugin;
use spring_sa_token::{SaTokenPlugin, SaTokenAuthConfigurator};
use spring_web::{WebPlugin, WebConfigurator};

mod config;

#[auto_config(WebConfigurator)]
#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(RedisPlugin)       // with-spring-redis feature 需要
        .add_plugin(SaTokenPlugin)
        .add_plugin(WebPlugin)
        .sa_token_configure(config::SaTokenConfig)  // 配置路径认证
        .run()
        .await
}
```

### 2. 配置路径认证

`sa_token_configure()` 支持两种配置方式：

#### 方式一：使用 SaTokenConfig（推荐）

创建 `src/config.rs`：

```rust
use spring::app::AppBuilder;
use spring_sa_token::{PathAuthBuilder, SaStorage, SaTokenConfigurator};
use std::sync::Arc;

pub struct SaTokenConfig;

impl SaTokenConfigurator for SaTokenConfig {
    fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
        auth
            // 需要认证的路径
            .include("/user/**")
            .include("/admin/**")
            .include("/api/**")
            // 公开路径（无需认证）
            .exclude("/login")
            .exclude("/api/health")
    }
}
```

然后在 `main.rs` 中使用：

```rust
.sa_token_configure(config::SaTokenConfig)
```

#### 方式二：直接使用 PathAuthBuilder

你也可以直接在 `main.rs` 中配置，无需单独的配置文件：

```rust
use spring_sa_token::PathAuthBuilder;

#[tokio::main]
async fn main() {
    App::new()
        .add_plugin(RedisPlugin)
        .add_plugin(SaTokenPlugin)
        .add_plugin(WebPlugin)
        // 方式 2a：使用构建器模式
        .sa_token_configure(
            PathAuthBuilder::new()
                .include("/user/**")
                .include("/admin/**")
                .include("/api/**")
                .exclude("/login")
                .exclude("/public/**")
                .exclude("/api/health"),
        )
        // 方式 2b：使用结构体字面量
        // .sa_token_auth(PathAuthBuilder {
        //     include: vec![
        //         "/user/**".to_string(),
        //         "/admin/**".to_string(),
        //     ],
        //     exclude: vec![
        //         "/login".to_string(),
        //         "/public/**".to_string(),
        //     ],
        // })
        .run()
        .await
}
```

**路径匹配规则：**
- `**` 匹配任意多级路径，如 `/api/**` 匹配 `/api/users`、`/api/users/123` 等
- `*` 匹配单级路径，如 `/api/*` 只匹配 `/api/users`，不匹配 `/api/users/123`
- 精确匹配，如 `/login` 只匹配 `/login`

### 3. 实现登录接口

```rust
use spring_sa_token::StpUtil;
use spring_web::{post, axum::response::IntoResponse, extractor::Json, error::Result};

#[post("/login")]
async fn login(Json(req): Json<LoginRequest>) -> Result<impl IntoResponse> {
    // 验证凭证（你的业务逻辑）
    if req.username == "admin" && req.password == "123456" {
        // 登录并获取 Token
        let token = StpUtil::login(&req.username).await?;

        // 可选：设置角色和权限
        StpUtil::set_roles(&req.username, vec!["admin".to_string()]).await?;
        StpUtil::set_permissions(&req.username, vec!["user:list".to_string()]).await?;

        Ok(Json(LoginResponse {
            token: token.as_str().to_string(),
            message: "登录成功".to_string(),
        }))
    } else {
        Ok(Json(ErrorResponse { message: "凭证无效".to_string() }))
    }
}
```

### 4. 访问受保护的路由

```rust
use spring_sa_token::LoginIdExtractor;
use spring_web::{get, axum::response::IntoResponse, extractor::Json, error::Result};

#[get("/user/info")]
async fn user_info(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    // user_id 自动从 Token 中提取
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "message": format!("你好，{}！", user_id)
    })))
}
```

## 过程宏

`spring-sa-token` 提供了多个用于声明式安全的过程宏：

### `#[sa_check_login]`

验证用户已登录：

```rust
#[get("/api/profile")]
#[sa_check_login]
async fn get_profile(LoginIdExtractor(user_id): LoginIdExtractor) -> Result<impl IntoResponse> {
    Ok(Json(serde_json::json!({ "user_id": user_id })))
}
```

### `#[sa_check_role("role")]`

验证用户拥有指定角色：

```rust
#[get("/admin/dashboard")]
#[sa_check_role("admin")]
async fn admin_dashboard() -> impl IntoResponse {
    "欢迎来到管理后台"
}
```

### `#[sa_check_roles_and("role1", "role2")]`

验证用户拥有**所有**指定角色：

```rust
#[get("/api/super-admin")]
#[sa_check_roles_and("admin", "super")]
async fn super_admin_only() -> impl IntoResponse {
    "你同时拥有 admin 和 super 角色"
}
```

### `#[sa_check_roles_or("role1", "role2")]`

验证用户拥有**任意一个**指定角色：

```rust
#[get("/api/management")]
#[sa_check_roles_or("admin", "manager")]
async fn management_area() -> impl IntoResponse {
    "你拥有 admin 或 manager 角色"
}
```

### `#[sa_check_permission("permission")]`

验证用户拥有指定权限：

```rust
#[get("/admin/users")]
#[sa_check_permission("user:list")]
async fn list_users() -> impl IntoResponse {
    "用户列表"
}
```

### `#[sa_check_permissions_and("perm1", "perm2")]`

验证用户拥有**所有**指定权限：

```rust
#[post("/api/user/batch-modify")]
#[sa_check_permissions_and("user:edit", "user:delete")]
async fn batch_modify() -> impl IntoResponse {
    "批量修改成功"
}
```

### `#[sa_check_permissions_or("perm1", "perm2")]`

验证用户拥有**任意一个**指定权限：

```rust
#[post("/api/user/create-or-update")]
#[sa_check_permissions_or("user:add", "user:edit")]
async fn create_or_update() -> impl IntoResponse {
    "创建或更新成功"
}
```

### `#[sa_ignore]`

跳过特定端点的认证（即使路径匹配 include 规则）：

```rust
#[get("/api/health")]
#[sa_ignore]
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}
```

## StpUtil API

`StpUtil` 结构体提供了 Token 操作的静态方法：

### 登录/登出

```rust
// 登录并获取 Token
let token = StpUtil::login("user_id").await?;

// 登出当前 Token
StpUtil::logout("token").await?;

// 通过登录 ID 登出（使所有 Token 失效）
StpUtil::logout_by_login_id("user_id").await?;

// 检查用户是否已登录
let is_login = StpUtil::is_login_by_login_id("user_id").await;
```

### Token 操作

```rust
// 通过登录 ID 获取 Token
let token = StpUtil::get_token_by_login_id("user_id").await;

// 通过 Token 获取登录 ID
let login_id = StpUtil::get_login_id_by_token("token").await;
```

### 角色和权限

```rust
// 设置角色
StpUtil::set_roles("user_id", vec!["admin".to_string(), "user".to_string()]).await?;

// 获取角色
let roles = StpUtil::get_roles("user_id").await;

// 检查角色
let has_role = StpUtil::has_role("user_id", "admin").await;

// 设置权限
StpUtil::set_permissions("user_id", vec!["user:list".to_string()]).await?;

// 获取权限
let permissions = StpUtil::get_permissions("user_id").await;

// 检查权限
let has_perm = StpUtil::has_permission("user_id", "user:list").await;
```

## 提取器

### `LoginIdExtractor`

从请求中提取当前用户的登录 ID：

```rust
use spring_sa_token::LoginIdExtractor;

#[get("/user/info")]
async fn user_info(LoginIdExtractor(user_id): LoginIdExtractor) -> impl IntoResponse {
    format!("当前用户：{}", user_id)
}
```

### `OptionalSaTokenExtractor`

可选地提取 Token 信息（未认证时返回 None）：

```rust
use spring_sa_token::OptionalSaTokenExtractor;

#[get("/public")]
async fn public_endpoint(token: OptionalSaTokenExtractor) -> impl IntoResponse {
    match token.0 {
        Some(info) => format!("已登录：{}", info.login_id),
        None => "未登录".to_string(),
    }
}
```

### `SaTokenExtractor`

提取完整的 Token 信息（未认证时失败）：

```rust
use spring_sa_token::SaTokenExtractor;

#[get("/protected")]
async fn protected_endpoint(SaTokenExtractor(info): SaTokenExtractor) -> impl IntoResponse {
    format!("Token：{}，用户：{}", info.token, info.login_id)
}
```

## 组件访问

访问 `SaTokenState` 组件进行高级操作：

```rust
use spring_sa_token::SaTokenState;
use spring_web::extractor::Component;

#[get("/api/config")]
async fn get_config(Component(state): Component<SaTokenState>) -> impl IntoResponse {
    let config = &state.manager.config;
    Json(serde_json::json!({
        "token_name": config.token_name,
        "timeout": config.timeout,
        "token_style": format!("{:?}", config.token_style),
    }))
}
```

## 自定义存储

你可以使用 `lazy_storage<T>()` 实现自定义存储后端（如基于数据库）：

### 步骤 1：将你的存储定义为 Service

```rust
use spring::plugin::service::Service;
use spring_sa_token::SaStorage;
use spring_sea_orm::DbConn;
use sa_token_adapter::storage::{StorageResult, StorageError};

#[derive(Clone, Service)]
pub struct MyDatabaseStorage {
    #[inject(component)]
    conn: DbConn,  // 由 spring 框架自动注入
}

#[async_trait]
impl SaStorage for MyDatabaseStorage {
    async fn get(&self, key: &str) -> StorageResult<Option<String>> {
        // 你的数据库查询逻辑
        todo!()
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<i64>) -> StorageResult<()> {
        // 你的数据库插入/更新逻辑
        todo!()
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        // 你的数据库删除逻辑
        todo!()
    }

    // ... 实现其他必需的方法
}
```

### 步骤 2：在配置器中使用 lazy_storage

```rust
use spring::app::AppBuilder;
use spring_sa_token::{lazy_storage, PathAuthBuilder, SaStorage, SaTokenConfigurator};
use std::sync::Arc;

pub struct SaTokenConfig;

impl SaTokenConfigurator for SaTokenConfig {
    fn configure_path_auth(&self, auth: PathAuthBuilder) -> PathAuthBuilder {
        auth.include("/api/**").exclude("/login")
    }

    fn configure_storage(&self, _app: &AppBuilder) -> Option<Arc<dyn SaStorage>> {
        // 使用 lazy_storage 包装你基于 Service 的存储
        // 依赖项（如 DbConn）会在运行时自动注入
        Some(lazy_storage::<MyDatabaseStorage>())
    }
}
```

`lazy_storage<T>()` 函数：
- 使用懒加载组件解析包装你的 `#[derive(Service)]` 存储
- 依赖项在首次使用存储时自动注入
- 无需手动处理 `LazyComponent` - 框架会在内部处理

## 错误处理

所有安全宏在失败时返回 `spring_web::error::WebError`，可以由你的错误处理中间件处理：

```rust
use spring_web::error::Result;

#[get("/admin/dashboard")]
#[sa_check_role("admin")]
async fn admin_dashboard() -> Result<impl IntoResponse> {
    // 如果用户没有 "admin" 角色，返回 403 Forbidden
    Ok(Json(serde_json::json!({ "message": "欢迎！" })))
}
```

完整代码参考 [`sa-token-example`][sa-token-example]

[sa-token-example]: https://github.com/spring-rs/spring-rs/tree/master/examples/sa-token-example