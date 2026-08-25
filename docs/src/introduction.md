# Bouldy

Bouldy is a Rust workspace for runtime modding of Unreal Engine games. It focuses on in-memory interaction with a running game: lifecycle callbacks, runtime logic, and a small C++ bridge that can be wired into UE4SS.

The project is intentionally narrow. It does not parse assets, build `.pak` files, generate Unreal SDK bindings, or encode game-specific behavior.

## What Bouldy Provides

- A minimal C ABI between a loader shim and Rust mods.
- A safe Rust `Mod` trait for initialization, ticking, and shutdown.
- A proc macro that generates the exported Rust entrypoints.
- An example `cdylib` mod that can be loaded by a C++ UE4SS-side shim.
- Nix and Cargo workflows for native validation and Windows DLL builds.

## Relationship To Offline Tooling

Bouldy is complementary to projects such as `AstroTechies/unrealmodding`, which focus on asset parsing, `.pak` handling, and offline pipelines. Bouldy stays on the runtime side.

Source code is hosted on [GitHub](https://github.com/caniko/rs-bouldy).
