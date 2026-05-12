# Circular Dependency Injection Example

This example demonstrates how to handle circular dependencies between services using `LazyComponent`.

When two services reference each other, Rust's type system prevents direct circular dependencies:

```rust
// ❌ This will fail with "recursive type has infinite size"
#[derive(Clone, Service)]
struct UserService {
    #[inject(component)]
    better_user: BetterUserService,  // ERROR!
}

#[derive(Clone, Service)]
struct BetterUserService {
    #[inject(component)]
    user_service: UserService,  // ERROR!
}
```

The most common error you'll see is:

```
error[E0072]: recursive types `UserService` and `BetterUserService` have infinite size
```

For solve this you can use `LazyComponent<T>` to break the circular dependency:

```rust
use summer::plugin::LazyComponent;

#[derive(Clone, Service)]
struct UserService {
    better_user: LazyComponent<BetterUserService>,  // ✅ Lazy resolution
}

#[derive(Clone, Service)]
struct BetterUserService {
    #[inject(component)]
    user_service: UserService,  // ✅ Direct injection OK
}
```

This allows `UserService` to hold a lightweight reference to `BetterUserService` that is only resolved when needed, but this will be solved in runtime, just call `.get()` to access the actual component.

For do this the attribute `#[inject]` is not needed, `LazyComponent<T>` is automatically detected.
Internally this type is just a wrapper around `Arc<RwLock<...>>` so it is thread-safe.

Only one side of the circular dependency needs to be lazy, so choose the less frequently accessed service to be lazy but both can be lazy if needed.
