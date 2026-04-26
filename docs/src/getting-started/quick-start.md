# Quick Start

Build and validate the workspace:

```bash
nix develop
cargo test --workspace
```

Build the example Windows DLL:

```bash
nix develop .#windows
cargo build -p example-mod --release --target x86_64-pc-windows-gnu
```

The DLL is written to:

```text
target/x86_64-pc-windows-gnu/release/example_mod.dll
```

Place that DLL next to the C++ shim in your UE4SS mod layout. The shim dynamically loads the Rust DLL and calls `bouldy_rust_init_v1`.

## Minimal Rust Mod

```rust
use bouldy_runtime::prelude::*;

#[unreal_mod]
#[derive(Default)]
struct MyMod;

impl Mod for MyMod {
    fn on_init(&mut self, ctx: &mut ModContext) {
        ctx.log("Rust mod initialized");
    }

    fn on_tick(&mut self, delta: f32) {
        let _ = delta;
    }
}
```

