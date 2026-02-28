//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

mod auto;
mod cache;
mod component;
mod config;
mod problem_details;
mod inject;
mod job;
mod middlewares;
mod nest;
mod route;
#[cfg(feature = "socket_io")]
mod socketioxide;
mod stream;
mod utils;

#[cfg(feature = "sa-token")]
mod sa_token;

use proc_macro::TokenStream;
use syn::DeriveInput;

/// Creates resource handler.
///
/// # Syntax
/// ```plain
/// #[route("path", method="HTTP_METHOD"[, attributes])]
/// ```
///
/// # Attributes
/// - `"path"`: Raw literal string with path for which to register handler.
/// - `method = "HTTP_METHOD"`: Registers HTTP method to provide guard for. Upper-case string,
///   "GET", "POST" for example.
///
/// # Examples
/// ```
/// # use summer_web::axum::response::IntoResponse;
/// # use summer_macros::route;
/// #[route("/test", method = "GET", method = "HEAD")]
/// async fn example() -> impl IntoResponse {
///     "hello world"
/// }
/// ```
#[proc_macro_attribute]
pub fn route(args: TokenStream, input: TokenStream) -> TokenStream {
    route::with_method(None, args, input, false)
}

/// Creates openapi resource handler.
///
/// # Syntax
/// ```plain
/// #[api_route("path", method="HTTP_METHOD"[, attributes])]
/// ```
///
/// # Attributes
/// - `"path"`: Raw literal string with path for which to register handler.
/// - `method = "HTTP_METHOD"`: Registers HTTP method. Upper-case string,
///   "GET", "POST" for example.
///
/// # Examples
/// ```
/// # use summer_web::axum::response::IntoResponse;
/// # use summer_macros::api_route;
/// #[api_route("/test", method = "GET", method = "HEAD")]
/// async fn example() -> impl IntoResponse {
///     "hello world"
/// }
/// ```
#[proc_macro_attribute]
pub fn api_route(args: TokenStream, input: TokenStream) -> TokenStream {
    route::with_method(None, args, input, true)
}

/// Creates resource handler.
///
/// # Syntax
/// ```plain
/// #[routes]
/// #[<method>("path", ...)]
/// #[<method>("path", ...)]
/// ...
/// ```
///
/// # Attributes
/// The `routes` macro itself has no parameters, but allows specifying the attribute macros for
/// the multiple paths and/or methods, e.g. [`GET`](macro@get) and [`POST`](macro@post).
///
/// These helper attributes take the same parameters as the [single method handlers](crate#single-method-handler).
///
/// # Examples
/// ```
/// # use summer_web::axum::response::IntoResponse;
/// # use summer_macros::routes;
/// #[routes]
/// #[get("/test")]
/// #[get("/test2")]
/// #[delete("/test")]
/// async fn example() -> impl IntoResponse {
///     "hello world"
/// }
/// ```
#[proc_macro_attribute]
pub fn routes(_: TokenStream, input: TokenStream) -> TokenStream {
    route::with_methods(input, false)
}

/// Creates openapi resource handler.
///
/// # Syntax
/// ```plain
/// #[api_routes]
/// #[<method>("path", ...)]
/// #[<method>("path", ...)]
/// ...
/// ```
///
/// # Attributes
/// The `api_routes` macro itself has no parameters, but allows specifying the attribute macros for
/// the multiple paths and/or methods, e.g. [`GET`](macro@get) and [`POST`](macro@post).
///
/// These helper attributes take the same parameters as the [single method handlers](crate#single-method-handler).
///
/// # Examples
/// ```
/// # use summer_web::axum::response::IntoResponse;
/// # use summer_macros::api_routes;
/// #[api_routes]
/// #[get("/test")]
/// #[get("/test2")]
/// #[delete("/test")]
/// async fn example() -> impl IntoResponse {
///     "hello world"
/// }
/// ```
#[proc_macro_attribute]
pub fn api_routes(_: TokenStream, input: TokenStream) -> TokenStream {
    route::with_methods(input, true)
}

macro_rules! method_macro {
    ($variant:ident, $method:ident, $openapi:expr) => {
        ///
        /// # Syntax
        /// ```plain
        #[doc = concat!("#[", stringify!($method), r#"("path"[, attributes])]"#)]
        /// ```
        ///
        /// # Attributes
        /// - `"path"`: Raw literal string with path for which to register handler.
        ///
        /// # Examples
        /// ```
        /// # use summer_web::axum::response::IntoResponse;
        #[doc = concat!("# use summer_macros::", stringify!($method), ";")]
        #[doc = concat!("#[", stringify!($method), r#"("/")]"#)]
        /// async fn example() -> impl IntoResponse {
        ///     "hello world"
        /// }
        /// ```
        #[proc_macro_attribute]
        pub fn $method(args: TokenStream, input: TokenStream) -> TokenStream {
            route::with_method(Some(route::Method::$variant), args, input, $openapi)
        }
    };
}

method_macro!(Get, get, false);
method_macro!(Post, post, false);
method_macro!(Put, put, false);
method_macro!(Delete, delete, false);
method_macro!(Head, head, false);
method_macro!(Options, options, false);
method_macro!(Trace, trace, false);
method_macro!(Patch, patch, false);

method_macro!(Get, get_api, true);
method_macro!(Post, post_api, true);
method_macro!(Put, put_api, true);
method_macro!(Delete, delete_api, true);
method_macro!(Head, head_api, true);
method_macro!(Options, options_api, true);
method_macro!(Trace, trace_api, true);
method_macro!(Patch, patch_api, true);

/// Prepends a path prefix to all handlers using routing macros inside the attached module.
///
/// # Syntax
///
/// ```
/// # use summer_macros::nest;
/// #[nest("/prefix")]
/// mod api {
///     // ...
/// }
/// ```
///
/// # Arguments
///
/// - `"/prefix"` - Raw literal string to be prefixed onto contained handlers' paths.
///
/// # Example
///
/// ```
/// # use summer_macros::{nest, get};
/// # use summer_web::axum::response::IntoResponse;
/// #[nest("/api")]
/// mod api {
///     # use super::*;
///     #[get("/hello")]
///     pub async fn hello() -> impl IntoResponse {
///         // this has path /api/hello
///         "Hello, world!"
///     }
/// }
/// # fn main() {}
/// ```
#[proc_macro_attribute]
pub fn nest(args: TokenStream, input: TokenStream) -> TokenStream {
    nest::with_nest(args, input)
}

/// Applies middleware layers to all route handlers within a module.
///
/// # Syntax
/// ```plain
/// #[middlewares(middleware1, middleware2, ...)]
/// mod module_name {
///     // route handlers
/// }
/// ```
///
/// # Arguments
/// - `middleware1`, `middleware2`, etc. - Middleware expressions that will be applied to all routes in the module
///
/// This macro generates a router function that applies the specified middleware
/// to all route handlers defined within the module.
#[proc_macro_attribute]
pub fn middlewares(args: TokenStream, input: TokenStream) -> TokenStream {
    middlewares::middlewares(args, input)
}

fn input_and_compile_error(mut item: TokenStream, err: syn::Error) -> TokenStream {
    let compile_err = TokenStream::from(err.to_compile_error());
    item.extend(compile_err);
    item
}

/// Job
///
macro_rules! job_macro {
    ($variant:ident, $job_type:ident, $example:literal) => {
        ///
        /// # Syntax
        /// ```plain
        #[doc = concat!("#[", stringify!($job_type), "(", $example, ")]")]
        /// ```
        ///
        /// # Attributes
        /// - `"path"`: Raw literal string with path for which to register handler.
        ///
        /// # Examples
        /// ```
        /// # use summer_web::axum::response::IntoResponse;
        #[doc = concat!("# use summer_macros::", stringify!($job_type), ";")]
        #[doc = concat!("#[", stringify!($job_type), "(", stringify!($example), ")]")]
        /// async fn example() {
        ///     println!("hello world");
        /// }
        /// ```
        #[proc_macro_attribute]
        pub fn $job_type(args: TokenStream, input: TokenStream) -> TokenStream {
            job::with_job(job::JobType::$variant, args, input)
        }
    };
}

job_macro!(OneShot, one_shot, 60);
job_macro!(FixDelay, fix_delay, 60);
job_macro!(FixRate, fix_rate, 60);
job_macro!(Cron, cron, "1/10 * * * * *");

/// Auto config
/// ```diff
///  use summer_macros::auto_config;
///  use summer_web::{WebPlugin, WebConfigurator};
///  use summer_job::{JobPlugin, JobConfigurator};
///  use summer_boot::app::App;
/// +#[auto_config(WebConfigurator, JobConfigurator)]
///  #[tokio::main]
///  async fn main() {
///      App::new()
///         .add_plugin(WebPlugin)
///         .add_plugin(JobPlugin)
/// -       .add_router(router())
/// -       .add_jobs(jobs())
///         .run()
///         .await
///  }
/// ```
///
#[proc_macro_attribute]
pub fn auto_config(args: TokenStream, input: TokenStream) -> TokenStream {
    auto::config(args, input)
}

/// stream macro
#[proc_macro_attribute]
pub fn stream_listener(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::listener(args, input)
}

/// Configurable
#[proc_macro_derive(Configurable, attributes(config_prefix))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    config::expand_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Injectable Servcie
#[proc_macro_derive(Service, attributes(service, inject))]
pub fn derive_service(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    inject::expand_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Component macro for declarative component registration
///
/// This macro allows you to register components to the summer-rs application
/// container in a declarative way, without manually implementing the Plugin trait.
///
/// # Syntax
/// ```plain
/// #[component]
/// fn create_component(
///     Config(config): Config<MyConfig>,
///     Component(dep): Component<Dependency>,
/// ) -> MyComponent {
///     MyComponent::new(config, dep)
/// }
/// Declarative component registration macro for summer-rs applications.
///
/// The `#[component]` macro automatically generates a Plugin implementation that registers
/// a component in the application. It handles dependency injection, configuration loading,
/// and proper initialization order.
///
/// # How It Works
///
/// The macro transforms a component creation function into a Plugin that:
/// 1. Extracts dependencies from function parameters
/// 2. Generates a unique Plugin name based on the return type
/// 3. Declares dependencies to ensure correct initialization order
/// 4. Registers the component in the application registry
/// 5. Automatically registers the Plugin via the `inventory` crate
///
/// # Syntax
/// ```plain
/// #[component(name = "CustomName")]  // Optional custom name
/// fn create_component(
///     Config(config): Config<ConfigType>,      // Configuration injection
///     Component(dep): Component<DependencyType>, // Component injection
/// ) -> ComponentType {
///     // Component creation logic
/// }
/// ```
///
/// # Attributes
/// - `name = "PluginName"` - **Optional**: Custom Plugin name. If not specified, the name
///   is automatically generated as `__Create{TypeName}Plugin`.
///
/// # Parameters
/// - `Config<T>` - Inject configuration of type `T` (must implement `Configurable`)
/// - `Component<T>` - Inject another component of type `T`
///   - Can use `#[inject("PluginName")]` attribute to specify explicit dependency
///   - Without `#[inject]`, dependency is inferred as `__Create{T}Plugin`
///
/// # Return Type
/// - Must implement `Clone + Send + Sync + 'static`
/// - Can return `Result<T, E>` for fallible initialization (will panic on error)
/// - Each component type can only be registered once
///
/// # Dependency Resolution
///
/// The macro automatically analyzes dependencies and generates a `dependencies()` method
/// that returns the list of required Plugin names. The application will initialize plugins
/// in the correct order based on these dependencies.
///
/// **Circular dependencies are not allowed** and will cause a panic at runtime.
///
/// # Examples
///
/// ## Basic Usage
/// ```rust,ignore
/// use summer::config::Configurable;
/// use summer::extractor::Config;
/// use summer_macros::component;
/// use serde::Deserialize;
///
/// #[derive(Clone, Configurable, Deserialize)]
/// #[config_prefix = "database"]
/// struct DbConfig {
///     host: String,
///     port: u16,
/// }
///
/// #[derive(Clone)]
/// struct DbConnection {
///     url: String,
/// }
///
/// #[component]
/// fn create_db_connection(
///     Config(config): Config<DbConfig>,
/// ) -> DbConnection {
///     DbConnection {
///         url: format!("{}:{}", config.host, config.port),
///     }
/// }
/// ```
///
/// ## With Dependencies
/// ```rust,ignore
/// #[derive(Clone)]
/// struct UserRepository {
///     db: DbConnection,
/// }
///
/// #[component]
/// fn create_user_repository(
///     Component(db): Component<DbConnection>,
/// ) -> UserRepository {
///     UserRepository { db }
/// }
/// ```
///
/// ## Multi-Level Dependencies
/// ```rust,ignore
/// #[derive(Clone)]
/// struct UserService {
///     repo: UserRepository,
/// }
///
/// #[component]
/// fn create_user_service(
///     Component(repo): Component<UserRepository>,
/// ) -> UserService {
///     UserService { repo }
/// }
/// ```
///
/// ## Async Initialization
/// ```rust,ignore
/// #[component]
/// async fn create_db_connection(
///     Config(config): Config<DbConfig>,
/// ) -> Result<DbConnection, anyhow::Error> {
///     let pool = sqlx::PgPool::connect(&config.url).await?;
///     Ok(DbConnection { pool })
/// }
/// ```
///
/// ## Custom Plugin Name
///
/// Use custom names when you need multiple components of the same type (NewType pattern):
/// ```rust,ignore
/// #[derive(Clone)]
/// struct PrimaryDb(DbConnection);
///
/// #[derive(Clone)]
/// struct SecondaryDb(DbConnection);
///
/// #[component(name = "PrimaryDatabase")]
/// fn create_primary_db(
///     Config(config): Config<PrimaryDbConfig>,
/// ) -> PrimaryDb {
///     PrimaryDb(DbConnection::new(&config))
/// }
///
/// #[component(name = "SecondaryDatabase")]
/// fn create_secondary_db(
///     Config(config): Config<SecondaryDbConfig>,
/// ) -> SecondaryDb {
///     SecondaryDb(DbConnection::new(&config))
/// }
/// ```
///
/// ## Explicit Dependency
///
/// Use `#[inject("PluginName")]` when the dependency name cannot be inferred:
/// ```rust,ignore
/// #[component]
/// fn create_repository(
///     #[inject("PrimaryDatabase")] Component(db): Component<PrimaryDb>,
/// ) -> UserRepository {
///     UserRepository::new(db.0)
/// }
/// ```
///
/// # Usage in Application
///
/// Components defined with `#[component]` are automatically registered when the app is built:
///
/// ```rust,ignore
/// use summer::App;
///
/// #[tokio::main]
/// async fn main() {
///     let app = App::new()
///         .build()  // Auto plugins are registered automatically
///         .await
///         .expect("Failed to build app");
///     
///     // Use components
///     let db = app.get_component::<DbConnection>().unwrap();
/// }
/// ```
///
/// # Best Practices
///
/// 1. **Keep component functions simple** - They should only create and configure the component
/// 2. **Use NewType pattern for multiple instances** - Wrap the same type in different structs
/// 3. **Prefer configuration over hardcoding** - Use `Config<T>` for all configurable values
/// 4. **Use `Arc<T>` for large components** - Reduces clone overhead
/// 5. **Avoid circular dependencies** - Refactor your design if you encounter them
/// 6. **Use explicit names for clarity** - When the auto-generated name is not clear enough
///
/// # Limitations
///
/// - Each component type can only be registered once (use NewType pattern for multiple instances)
/// - Circular dependencies are not supported
/// - Component types must implement `Clone + Send + Sync + 'static`
/// - Configuration types must implement `Configurable + Deserialize`
///
/// # See Also
///
/// - [`Config`](summer::extractor::Config) - Configuration injection wrapper
/// - [`Component`](summer::extractor::Component) - Component injection wrapper
/// - [`Configurable`](summer::config::Configurable) - Trait for configuration types
#[proc_macro_attribute]
pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    component::component_macro(attr, input)
}

/// ProblemDetails derive macro
///
/// Derives the `From<T> for ProblemDetails` trait for error enums.
/// This macro automatically generates implementations for converting error variants
/// to RFC 7807 Problem Details responses.
///
/// Each variant must have a `#[status_code(code)]` attribute.
/// 
/// ## Supported Attributes
/// 
/// - `#[status_code(code)]` - **Required**: HTTP status code (e.g., 400, 404, 500)
/// - `#[problem_type("uri")]` - **Optional**: Custom problem type URI
/// - `#[title("title")]` - **Optional**: Custom problem title
/// - `#[detail("detail")]` - **Optional**: Custom problem detail message
/// - `#[instance("uri")]` - **Optional**: Problem instance URI
///
/// ## Title Compatibility
/// 
/// The `title` field can be automatically derived from the `#[error("...")]` attribute
/// if no explicit `#[title("...")]` is provided. This provides compatibility with
/// `thiserror::Error` and reduces duplication.
///
/// ## Basic Example
/// ```rust,ignore
/// use summer_web::ProblemDetails;
///
/// #[derive(ProblemDetails)]
/// pub enum ApiError {
///     #[status_code(400)]
///     ValidationError,
///     #[status_code(404)]
///     NotFound,
///     #[status_code(500)]
///     InternalError,
/// }
/// ```
///
/// ## Advanced Example with Custom Attributes
/// ```rust,ignore
/// #[derive(ProblemDetails)]
/// pub enum ApiError {
///     // Explicit title
///     #[status_code(400)]
///     #[title("Input Validation Failed")]
///     #[detail("The provided input data is invalid")]
///     #[error("Validation error")]
///     ValidationError,
///     
///     // Title derived from error attribute
///     #[status_code(422)]
///     #[detail("Request data failed validation")]
///     #[error("Validation Failed")]  // This becomes the title
///     ValidationFailed,
///     
///     // Full customization
///     #[status_code(404)]
///     #[problem_type("https://api.example.com/problems/not-found")]
///     #[title("Resource Not Found")]
///     #[detail("The requested resource could not be found")]
///     #[instance("/users/123")]
///     #[error("Not found")]
///     NotFound,
/// }
/// ```
///
/// This will automatically implement:
/// - `From<T> for ProblemDetails` trait for converting to Problem Details responses
/// - `IntoResponse` trait for direct use in Axum handlers
/// - OpenAPI integration for documentation generation
#[proc_macro_derive(ProblemDetails, attributes(status_code, problem_type, title, detail, instance))]
pub fn derive_problem_details(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    problem_details::expand_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// `#[cache]` - Transparent Redis-based caching for async functions.
///
/// This macro wraps an async function to automatically cache its result
/// in Redis. It checks for a cached value before executing the function.
/// If a cached result is found, it is deserialized and returned directly.
/// Otherwise, the function runs normally and its result is stored in Redis.
///
/// # Syntax
/// ```plain
/// #[cache("key_pattern", expire = <seconds>, condition = <bool_expr>, unless = <bool_expr>)]
/// ```
///
/// # Attributes
/// - `"key_pattern"` (**required**):
///   A format string used to generate the cache key. Function arguments can be interpolated using standard `format!` syntax.
/// - `expire = <integer>` (**optional**):
///   The number of seconds before the cached value expires. If omitted, the key will be stored without expiration.
/// - `condition = <expression>` (**optional**):
///   A boolean expression evaluated **before** executing the function.
///   If this evaluates to `false`, caching is completely bypassed — no lookup and no insertion.
///   The expression can access function parameters directly.
/// - `unless = <expression>` (**optional**):
///   A boolean expression evaluated **after** executing the function.
///   If this evaluates to `true`, the result will **not** be written to the cache.
///   The expression can access both parameters and a `result` variable (the return value).
///   NOTE: If your function returns Result<T, E>, the `result` variable in unless refers to the inner Ok value (T), not the entire Result.
///   This allows you to write expressions like result.is_none() for Result<Option<_>, _> functions.
///
/// # Function Requirements
/// - Must be an `async fn`
/// - Can return either a `Result<T, E>` or a plain value `T`
/// - The return type must implement `serde::Serialize` and `serde::Deserialize`
/// - Generics, attributes, and visibility will be preserved
///
/// # Example
/// ```rust
/// use summer_macros::cache;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// struct MyError;
///
/// #[cache("user:{user_id}", expire = 600, condition = user_id % 2 == 0, unless = result.is_none())]
/// async fn get_user(user_id: u64) -> Result<Option<User>, MyError> {
///     // Fetch user from database
///     unimplemented!("do something")
/// }
/// ```
#[proc_macro_attribute]
pub fn cache(args: TokenStream, input: TokenStream) -> TokenStream {
    cache::cache(args, input)
}

#[cfg(feature = "socket_io")]
/// Marks a function as a SocketIO connection handler
///
/// # Examples
/// ```
/// # use summer_web::socketioxide::extract::{SocketRef, Data};
/// # use summer_web::rmpv::Value;
/// # use summer_macros::on_connection;
/// #[on_connection]
/// async fn on_connection(socket: SocketRef, Data(data): Data<Value>) {
///     // Handle connection
/// }
/// ```
#[proc_macro_attribute]
pub fn on_connection(args: TokenStream, input: TokenStream) -> TokenStream {
    socketioxide::on_connection(args, input)
}

#[cfg(feature = "socket_io")]
/// Marks a function as a SocketIO disconnection handler
///
/// # Examples
/// ```
/// # use summer_web::socketioxide::extract::SocketRef;
/// # use summer_macros::on_disconnect;
/// #[on_disconnect]
/// async fn on_disconnect(socket: SocketRef) {
///     // Handle disconnection
/// }
/// ```
#[proc_macro_attribute]
pub fn on_disconnect(args: TokenStream, input: TokenStream) -> TokenStream {
    socketioxide::on_disconnect(args, input)
}

#[cfg(feature = "socket_io")]
/// Marks a function as a SocketIO message subscription handler
///
/// # Examples
/// ```
/// # use summer_web::socketioxide::extract::{SocketRef, Data};
/// # use summer_macros::subscribe_message;
/// # use summer_web::rmpv::Value;
/// #[subscribe_message("message")]
/// async fn message(socket: SocketRef, Data(data): Data<Value>) {
///     // Handle message
/// }
/// ```
#[proc_macro_attribute]
pub fn subscribe_message(args: TokenStream, input: TokenStream) -> TokenStream {
    socketioxide::subscribe_message(args, input)
}

#[cfg(feature = "socket_io")]
/// Marks a function as a SocketIO fallback handler
///
/// # Examples
/// ```
/// # use summer_web::socketioxide::extract::{SocketRef, Data};
/// # use summer_web::rmpv::Value;
/// # use summer_macros::on_fallback;
/// #[on_fallback]
/// async fn on_fallback(socket: SocketRef, Data(data): Data<Value>) {
///     // Handle fallback
/// }
/// ```
#[proc_macro_attribute]
pub fn on_fallback(args: TokenStream, input: TokenStream) -> TokenStream {
    socketioxide::on_fallback(args, input)
}

// ============================================================================
// Sa-Token authentication macros
// ============================================================================

#[cfg(feature = "sa-token")]
/// Check login status
///
/// Returns 401 Unauthorized if user is not logged in.
///
/// # Example
/// ```rust,ignore
/// #[sa_check_login]
/// async fn user_info() -> Result<impl IntoResponse> {
///     Ok("User info")
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_check_login(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_check_login_impl(attr, input)
}

#[cfg(feature = "sa-token")]
/// Check user role
///
/// Returns 401 if not logged in, 403 Forbidden if user doesn't have the required role.
///
/// # Example
/// ```rust,ignore
/// #[sa_check_role("admin")]
/// async fn admin_panel() -> Result<impl IntoResponse> {
///     Ok("Admin panel")
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_check_role(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_check_role_impl(attr, input)
}

#[cfg(feature = "sa-token")]
/// Check user permission
///
/// Returns 401 if not logged in, 403 Forbidden if user doesn't have the required permission.
///
/// # Example
/// ```rust,ignore
/// #[sa_check_permission("user:delete")]
/// async fn delete_user() -> Result<impl IntoResponse> {
///     Ok("User deleted")
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_check_permission(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_check_permission_impl(attr, input)
}

#[cfg(feature = "sa-token")]
/// Check multiple roles with AND logic
///
/// User must have ALL specified roles to access.
///
/// # Example
/// ```rust,ignore
/// #[sa_check_roles_and("admin", "super")]
/// async fn super_admin() -> Result<impl IntoResponse> {
///     Ok("Super admin")
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_check_roles_and(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_check_roles_and_impl(attr, input)
}

#[cfg(feature = "sa-token")]
/// Check multiple roles with OR logic
///
/// User must have ANY of the specified roles to access.
///
/// # Example
/// ```rust,ignore
/// #[sa_check_roles_or("admin", "manager")]
/// async fn management() -> Result<impl IntoResponse> {
///     Ok("Management area")
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_check_roles_or(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_check_roles_or_impl(attr, input)
}

#[cfg(feature = "sa-token")]
/// Check multiple permissions with AND logic
///
/// User must have ALL specified permissions to access.
///
/// # Example
/// ```rust,ignore
/// #[sa_check_permissions_and("user:read", "user:write")]
/// async fn user_rw() -> Result<impl IntoResponse> {
///     Ok("User read/write")
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_check_permissions_and(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_check_permissions_and_impl(attr, input)
}

#[cfg(feature = "sa-token")]
/// Check multiple permissions with OR logic
///
/// User must have ANY of the specified permissions to access.
///
/// # Example
/// ```rust,ignore
/// #[sa_check_permissions_or("admin:*", "user:delete")]
/// async fn delete() -> Result<impl IntoResponse> {
///     Ok("Delete operation")
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_check_permissions_or(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_check_permissions_or_impl(attr, input)
}

#[cfg(feature = "sa-token")]
/// Ignore authentication for this endpoint
///
/// This macro marks an endpoint to skip authentication checks,
/// even if it's under a path that normally requires authentication.
///
/// # Example
/// ```rust,ignore
/// #[sa_ignore]
/// async fn public_endpoint() -> impl IntoResponse {
///     "This endpoint is public"
/// }
/// ```
#[proc_macro_attribute]
pub fn sa_ignore(attr: TokenStream, input: TokenStream) -> TokenStream {
    sa_token::sa_ignore_impl(attr, input)
}
