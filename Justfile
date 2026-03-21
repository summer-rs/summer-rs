# Summer-RS Development Commands
# Run 'just' or 'just --list' to see all available commands

# Default recipe - show help
default:
    @just --list

# Run all tests
test:
    cargo test --workspace --all-features

# Run all tests with verbose output
test-verbose:
    cargo test --workspace --all-features -- --nocapture

# Run unit tests only
test-unit:
    cargo test --lib --workspace

# Run integration tests only
test-integration:
    cargo test --test '*' --workspace

# Run doc tests
test-doc:
    cargo test --doc --workspace

# Run a specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# Generate test coverage with tarpaulin
test-coverage:
    @echo "Generating test coverage..."
    cargo tarpaulin --workspace --out Html --output-dir coverage --timeout 300
    @echo "Coverage report generated in ./coverage/index.html"

# Generate test coverage with llvm-cov
test-coverage-llvm:
    @echo "Generating test coverage with llvm-cov..."
    cargo llvm-cov --workspace --html
    @echo "Coverage report generated"

# Run tests in watch mode (requires cargo-watch)
test-watch:
    cargo watch -x "test --workspace"

# Format code
fmt:
    cargo fmt --all

# Check formatting without making changes
fmt-check:
    cargo fmt --all -- --check

# Run clippy
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run cargo check
check:
    cargo check --workspace --all-targets --all-features

# Build all packages
build:
    cargo build --workspace

# Build release version
build-release:
    cargo build --workspace --release

# Build documentation
docs:
    cargo doc --workspace --no-deps --open

# Clean build artifacts
clean:
    cargo clean
    rm -rf coverage/
    rm -rf target/

# Install development tools
install-tools:
    @echo "Installing development tools..."
    cargo install cargo-watch
    cargo install cargo-tarpaulin
    cargo install cargo-llvm-cov
    cargo install cargo-audit
    cargo install cargo-outdated
    @echo "Development tools installed!"

# Run security audit
audit:
    cargo audit

# Check for outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Run all checks (format, lint, test)
check-all: fmt-check lint test
    @echo "All checks passed!"

# Prepare for commit (format, lint, test)
pre-commit: fmt lint test
    @echo "Ready to commit!"

# Run benchmarks (if any)
bench:
    cargo bench --workspace

# Generate and open coverage report
coverage-open: test-coverage
    #!/usr/bin/env bash
    if command -v xdg-open > /dev/null; then
        xdg-open coverage/index.html
    elif command -v open > /dev/null; then
        open coverage/index.html
    else
        echo "Please open coverage/index.html manually"
    fi

# Run tests for a specific package
test-package PKG:
    cargo test --package {{PKG}}

# Test spring package
test-spring:
    cargo test --package spring

# Test spring-web package
test-spring-web:
    cargo test --package spring-web

# Test spring-redis package
test-spring-redis:
    cargo test --package spring-redis

# Test spring-sqlx package
test-spring-sqlx:
    cargo test --package spring-sqlx

# Test spring-job package
test-spring-job:
    cargo test --package spring-job

# Run all quality checks for CI
ci: fmt-check lint test
    @echo "CI checks completed successfully!"

