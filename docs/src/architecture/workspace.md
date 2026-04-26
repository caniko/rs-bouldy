# Workspace

Bouldy is organized as a small Cargo workspace:

- `bouldy-sys` contains raw C ABI types and unsafe bridge helpers.
- `bouldy-runtime` contains lifecycle state, panic containment, logging, and the safe `Mod` trait.
- `bouldy-macros` contains the `#[unreal_mod]` proc macro.
- `example-mod` builds a working Rust `cdylib` mod.
- `cpp-shim` contains a minimal UE4SS-side bridge example.

The crate split keeps raw pointers and FFI details out of the mod author API while leaving room for future loader integrations.

## Non-Goals

Bouldy does not include asset parsing, `.pak` building, Unreal SDK generation, reflection wrappers, or game-specific logic in the MVP.

