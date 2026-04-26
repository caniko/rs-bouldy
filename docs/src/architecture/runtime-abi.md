# Runtime ABI

The base API is intentionally small:

```rust
#[repr(C)]
pub struct UnrealApi {
    pub log: extern "C" fn(*const std::ffi::c_char),
    pub get_delta_seconds: extern "C" fn() -> f32,
}
```

`UnrealApiV1` embeds the base API at offset zero and adds optional lifecycle registration:

```rust
#[repr(C)]
pub struct UnrealApiV1 {
    pub base: UnrealApi,
    pub register_tick: Option<extern "C" fn(extern "C" fn(f32))>,
    pub register_shutdown: Option<extern "C" fn(extern "C" fn())>,
}
```

Rust mods export two entrypoints:

- `unreal_rust_init(UnrealApi*) -> bool`
- `bouldy_rust_init_v1(UnrealApiV1*) -> bool`

The compatibility entrypoint supports the base API. The V1 entrypoint lets a shim register Rust callbacks for tick and shutdown.

