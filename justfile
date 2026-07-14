# DokTUI — development command runner
# Install `just` from https://github.com/casey/just
# Or run the equivalent `cargo` commands directly.

# List available recipes
default:
    just --list

# Run the app locally
dev:
    cargo run

# Run the full test suite
test:
    cargo test --all-targets

# Fast type-check only
check:
    cargo check --all-targets

# Format code
fmt:
    cargo fmt

# Check that code is formatted without changing files
fmt-check:
    cargo fmt --check

# Run clippy linting
clippy:
    cargo clippy --all-targets

# Run the same checks as CI: clippy, type check, and tests
verify: clippy check test
    @echo "All verification checks passed"

# Format, lint, type-check, and test
lint: fmt clippy check test

# Build an optimized release binary
release:
    cargo build --release

# Build a static Linux musl binary (requires the target)
release-musl:
    cargo build --release --target x86_64-unknown-linux-musl
