# Bouldy

<!-- simit:badges:start -->

![CI](https://img.shields.io/badge/CI-managed-2088ff) [![docs](https://img.shields.io/badge/docs-enabled-6f42c1)](docs) [![crates.io](https://img.shields.io/badge/crates.io-ready-f46623)](https://crates.io/crates/bouldy-macros)

<!-- simit:badges:end -->

Bouldy is a minimal Rust workspace for runtime modding of Unreal Engine games. It focuses on in-memory lifecycle and logic integration through a small UE4SS-compatible C++ shim.

This MVP deliberately does not parse assets, build `.pak` files, generate Unreal SDK bindings, or include game-specific logic. Offline tooling remains the domain of projects such as `AstroTechies/unrealmodding`.

## Workspace

- `bouldy-sys`: raw C ABI types and unsafe bridge helpers.
- `bouldy-runtime`: safe lifecycle, logging, and mod trait.
- `bouldy-macros`: `#[unreal_mod]` entrypoint generation.
- `example-mod`: a working Rust `cdylib` mod.
- `cpp-shim`: a minimal C++ UE4SS bridge example.

## ABI

The base ABI is intentionally tiny:

```rust
#[repr(C)]
pub struct UnrealApi {
    pub log: extern "C" fn(*const std::ffi::c_char),
    pub get_delta_seconds: extern "C" fn() -> f32,
}
```

`UnrealApiV1` embeds this base and adds optional shim-owned lifecycle registration:

```rust
#[repr(C)]
pub struct UnrealApiV1 {
    pub base: UnrealApi,
    pub register_tick: Option<extern "C" fn(extern "C" fn(f32))>,
    pub register_shutdown: Option<extern "C" fn(extern "C" fn())>,
}
```

`UnrealApiV2` keeps `UnrealApiV1` as its first field and adds a separate
game-agnostic discovery interface:

```rust
#[repr(C)]
pub struct UnrealApiV2 {
    pub lifecycle: UnrealApiV1,
    pub discovery: UnrealDiscoveryApiV1,
}
```

The discovery API exposes filtered scans over broad Unreal concepts only:
objects, classes, functions, and properties. It is intended for reconnaissance
mods that need to discover candidate symbols before adding game-specific hooks.
It does not include SDK bindings, offsets, generated game headers, asset
parsing, or game-specific patching behavior.

Rust mods export:

- `unreal_rust_init(UnrealApi*) -> bool`
- `bouldy_rust_init_v1(UnrealApiV1*) -> bool`
- `bouldy_rust_init_v2(UnrealApiV2*) -> bool`

The V1 entrypoint lets the UE4SS shim register Rust tick and shutdown callbacks.
The V2 entrypoint lets the UE4SS shim provide both lifecycle callbacks and a
discovery backend.

## Example Mod

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

## Build

Native validation:

```bash
git add flake.nix flake.lock Cargo.toml Cargo.lock crates example-mod cpp-shim README.md rust-toolchain.toml .gitignore
nix develop
cargo check --workspace
cargo test --workspace
```

In a freshly initialized git repository, Nix flakes only see tracked files. Stage the bootstrap files before running `nix develop`; committing is not required for local validation.

Windows DLL build:

```bash
nix develop .#windows
cargo build -p example-mod --release --target x86_64-pc-windows-gnu
```

The Windows DLL is produced at:

```text
target/x86_64-pc-windows-gnu/release/example_mod.dll
```

For non-Nix users, install Rust stable with the `x86_64-pc-windows-gnu` target and use the same Cargo commands.

## Website And Documentation

The Codeberg Pages site is split into a Zola landing page and mdBook documentation:

```bash
nix build .#website
nix build .#docs
nix build .#site
```

The combined `site` output serves the landing page at the root and documentation under `/docs/`.

For local authoring:

```bash
cd website && zola serve
cd docs && mdbook serve
```

## UE4SS Integration

The C++ shim lives in `cpp-shim/bouldy_ue4ss_shim.cpp`. Build it as the UE4SS-side module, place `example_mod.dll` beside it, and have the shim load the Rust DLL with `LoadLibraryW`.

The shim:

1. Resolves `bouldy_rust_init_v2`.
2. Provides logging and delta-time callbacks.
3. Provides tick/shutdown registration functions.
4. Provides discovery scan/export callbacks.
5. Forwards UE4SS lifecycle events into the registered Rust callbacks.

Dynamic DLL loading avoids MSVC/GNU import-library mismatches when the Rust DLL is cross-compiled with MinGW.

## Safety Model

Unsafe code is isolated to raw FFI, pointer validation, and C ABI calls in `bouldy-sys` and `bouldy-runtime`. Mod authors implement the safe `Mod` trait and use `ModContext` for runtime services.

The host shim owns the `UnrealApi`/`UnrealApiV1` table. That table must remain valid for the entire period in which Rust may call logging, delta-time, tick, or shutdown callbacks.

All C ABI entrypoints and registered callbacks are wrapped with `catch_unwind`, so Rust panics are contained and never unwind across C++ boundaries. Null API pointers are rejected, optional callbacks are checked before use, and C strings are sanitized before crossing FFI.

## Future Work

- `bouldy-reflection` for UObject/UClass/FName wrappers after ABI discovery is designed.
- `bouldy-hooks` for UE4SS/native hook registration.
- Optional interop with `unreal_asset` for tools that need offline asset data outside the runtime core.
