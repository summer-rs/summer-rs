# Web Middleware Example

This example demonstrates how to use the `#[middlewares]` macro in summer-RS to apply middleware layers to specific groups of routes. It showcases different middleware patterns including error handling, authentication, logging, and nested routing.

## Overview

The example includes three different middleware configurations:

1. **Basic Routes** (`/`, `/version`, `/error`) - Problem detail middleware with timeout
2. **Protected Routes** (`/protected`) - Authentication + logging middleware with CORS
3. **API Routes** (`/api/hello`) - Nested routes with authentication + logging middleware
4. **Standalone Route** (`/goodbye`) - No middleware applied

## Quick Start

### 1. Start Webserver
```sh
docker compose up -d
cargo run
```

### 2. Test the endpoints
The server will start on `http://localhost:8080`

## API Endpoints

### Basic Routes (Problem Detail Middleware)

| Endpoint | Method | Middleware | Auth Required |
|----------|--------|------------|---------------|
| `/` | GET | Problem Detail, Timeout | ❌ |
| `/version` | GET | Problem Detail, Timeout | ❌ |
| `/error` | GET | Problem Detail, Timeout | ❌ |

**Examples:**
```bash
# Success response
curl http://localhost:8080/
# Returns: "hello world"

# Database version
curl http://localhost:8080/version
# Returns: PostgreSQL version string

# Error demonstration (logged to database)
curl http://localhost:8080/error
# Returns: RFC 7807 Problem Detail JSON
```

### Protected Routes (Authentication + Logging)

| Endpoint | Method | Middleware | Auth Required |
|----------|--------|------------|---------------|
| `/protected` | GET | Logging, Auth, Timeout, CORS | ✅ |

**Examples:**
```bash
# Without authorization (401 Unauthorized)
curl http://localhost:8080/protected

# With authorization (200 OK)
curl -H "Authorization: Bearer any-token" http://localhost:8080/protected
# Returns: "Protected endpoint!"
```

### API Routes (Nested + Authentication)

| Endpoint     | Method | Module Middleware             | Route Middleware | Auth Required  |
|--------------|--------|------------------------------ |------------------| ---------------|
| `/api/hello` | GET    | Logging, Auth, Timeout, CORS  | Problem          | ✅             |
| `/api/error` | GET    | Logging, Auth, Timeout, CORS  |                  | ✅             |

**Examples:**
```bash
# Without authorization (401 Unauthorized)
curl http://localhost:8080/api/hello

# With authorization (400 Bad Request)  
curl -H "Authorization: Bearer any-token" http://localhost:8080/api/hello
# Returns: {"detail":"request error","instance":"/api/hello","title":"Bad Request"}

# With authorization (200 OK)  
curl -H "Authorization: Bearer any-token" http://localhost:8080/api/hello/Ferris
# Returns: Hello, Ferris!

# Without authorization (401 Unauthorized)
curl http://localhost:8080/api/error

# With authorization (500 Internal Server Error)  
curl -H "Authorization: Bearer any-token" http://localhost:8080/api/error 
# Returns: "error!"

# Unexisting API route (401 Unauthorized)
curl http://localhost:8080/api/unknown
# Returns: "Unauthorized"

# With authorization (404 Not Found)
curl -H "Authorization: Bearer any-token" http://localhost:8080/api/unknown
# Returns without body
# Status: 404 Not Found
# Logged in Console:
# 🔍 [LOGGING] GET /api/unknown
# 🔐 [AUTH] Checking authentication for: /api/unknown
# ✅ [LOGGING] Response completed
```

### Standalone Routes

| Endpoint         | Method | Middleware    | Auth Required |
|------------------|--------|---------------|---------------|
| `/goodbye`       | GET    | None          | ❌            |
| `/another_route` | GET    | Logging       | ❌            |

**Examples:**
```bash
# No middleware applied
curl http://localhost:8080/goodbye
# Returns: "goodbye world"

# Logged in Console:
# 🔍 [LOGGING] GET /another_route
# ✅ [LOGGING] Response completed
curl http://localhost:8080/goodbye
# Returns: "Another Route"
```

## Code Structure

```rust
// Module with middleware
#[middlewares(
    middleware::from_fn(problem_middleware),
    TimeoutLayer::new(Duration::from_secs(10))
)]
mod routes {
    #[get("/")]
    async fn hello_world() -> impl IntoResponse {
        "hello world"
    }
}

// Nested module with middleware
#[middlewares(
    middleware::from_fn(auth_middleware),
    // ... other middleware
)]
#[nest("/api")]  // All routes prefixed with /api
mod api {
    #[get("/hello")]  // Becomes /api/hello
    async fn hello() -> impl IntoResponse {
        "Hello, world!"
    }
}
```

## Database Setup

The example uses PostgreSQL for error logging. The middleware automatically creates error log entries when HTTP errors occur.

## Development Tools

**Generate database entities:**
```sh
sea-orm-cli generate entity --with-serde both --output-dir src/entities
```