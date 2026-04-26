# Testing

The MVP test suite covers:

- ABI layout expectations for `UnrealApiV1`.
- Null and no-op behavior in raw bridge helpers.
- Lifecycle registration through the V1 API.
- Panic containment for init, tick, and shutdown.
- Macro compile-pass and compile-fail behavior with `trybuild`.
- Windows DLL builds for the example mod.

Run the native suite:

```bash
nix develop
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run the Windows DLL build:

```bash
nix develop .#windows
cargo build -p example-mod --release --target x86_64-pc-windows-gnu
```

