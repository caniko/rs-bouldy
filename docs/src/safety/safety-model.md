# Safety Model

Bouldy treats the C ABI boundary as unsafe and keeps it in `bouldy-sys` and `bouldy-runtime`.

## Unsafe Surface

- Raw pointers passed from the host shim.
- Function pointers in `UnrealApi` and `UnrealApiV1`.
- Global runtime state used by registered callbacks.
- Host-owned callback and API table lifetimes.

## Safe Surface

Mod authors implement the safe `Mod` trait and receive a `ModContext` for runtime services.

```rust
pub trait Mod {
    fn on_init(&mut self, ctx: &mut ModContext) {}
    fn on_tick(&mut self, delta: f32) {}
    fn on_shutdown(&mut self) {}
}
```

## Panic Boundaries

All exported entrypoints and registered callbacks are wrapped with `catch_unwind`. A panic is logged when possible and does not unwind into C++.

## Host Lifetime Requirement

The host shim owns the `UnrealApi` or `UnrealApiV1` table. It must remain valid while Rust may log, read delta time, or receive lifecycle callbacks.

