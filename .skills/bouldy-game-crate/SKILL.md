---
name: bouldy-game-crate
description: Scaffold or update a game-specific UE4 modkit built on Bouldy for a particular Unreal Engine game. Use when creating per-game Rust mod crates, UE4SS/Bouldy loader shims, mirror-project notes, cooked asset mod workspace layout, runtime tick/init logic, or project flakes for Bouldy-based Unreal runtime mods. Always use the "rust project flake" skill to generate or update Nix flakes for Rust crates when that skill is available.
---

**Cross-repository work:** If scope spans repositories, invoke `$graphify` before discovery, planning, or edits. Query an existing graph; build/update a merged graph when missing, stale, or incomplete. Reuse a current graph for the same repository set.

# bouldy-game-crate

Create focused, production-lean UE4 modkits that use Bouldy as the native runtime layer for a specific Unreal Engine game.

A modkit may include a Rust runtime DLL, a tiny UE4SS-compatible loader/shim, optional editor/mirror-project guidance for cooked asset mods, and deployment docs. Keep those concerns separate so the Rust crate stays reliable and asset work stays in Unreal tooling.

## Core Rules

- Use this for game-specific modkits built on top of Bouldy, not for Bouldy core changes.
- Keep the Rust crate runtime-only: no asset parsing, no `.pak` building, no offline pipeline logic in the DLL.
- Put editor/project generation, SDK dumps, cooked content, and pak/chunk instructions in `modkit/`, `ue-project/`, `tools/`, or docs, not in `src/`.
- Put unsafe code behind a small internal boundary; expose safe game-mod APIs to the crate's own modules.
- Do not hardcode speculative offsets, object names, hook targets, or signatures. Add placeholders only when the user provides concrete game facts.
- Generate a `cdylib` mod crate that exports through Bouldy's `#[unreal_mod]` macro.
- Include docs for build, UE4SS/Bouldy deployment, known game/version assumptions, and where asset-mod workflows are intentionally only sketched.
- Treat game EULAs, anti-cheat, online play, encrypted assets, and redistribution of dumped headers/assets as explicit risk areas. Document assumptions; do not provide bypass instructions.

## Required Flake Workflow

When the Rust crate needs a new or updated `flake.nix`, use the skill named **rust project flake** first.

- If that skill is available, read and follow it before writing the flake.
- If that skill is not available in the current session, say so briefly and create the smallest rs-harbor-compatible flake by following existing local patterns.
- Preserve Windows cross-build support for `x86_64-pc-windows-gnu`.
- Include `cargo build --release --target x86_64-pc-windows-gnu` in shell hints.

## Discovery Checklist

Before editing:

- Identify whether this is a standalone repository or a workspace member.
- Inspect existing `Cargo.toml`, `flake.nix`, `README.md`, `.gitignore`, `src/`, and any `Mods/`, `modkit/`, `ue-project/`, or `tools/` directories.
- Find how Bouldy should be referenced:
  - local path dependency when developing beside `rs_bouldy`;
  - git dependency when the game crate is independent.
- Identify the game name, Unreal major/minor version if known, target platform, shipping executable folder, UE4SS install path, and loader path.
- Determine whether the request is runtime-only, asset-only, or hybrid:
  - runtime-only: Rust DLL plus loader docs;
  - asset-only: mirror project/cooking notes, no Rust crate unless requested;
  - hybrid: Rust DLL plus optional cooked `.pak`/chunk workspace.
- Check whether the game uses `.pak` only or Io Store (`.utoc`/`.ucas`), whether assets are encrypted, and whether there is official mod support.
- If the user has not provided concrete game/version details, use neutral placeholders and document them as assumptions.

## Default Modkit Shape

Use this shape for a standalone game modkit unless the repo already has conventions:

```text
<game>-bouldy-mod/
  Cargo.toml
  flake.nix
  README.md
  src/
    lib.rs
    game.rs
    logging.rs
    ue.rs
  loader/
    README.md
    ue4ss/
      mods.txt.example
  modkit/
    README.md
    asset-workflow.md
    mirror-project.md
  tools/
    README.md
```

For a workspace member, keep the same source layout but follow the workspace's existing package naming and dependency style. If the user asked only for a Rust crate, omit `modkit/` and `tools/`, but still document the deployment path.

## Efficient UE4 Modkit Workflow

Prefer this division of labor:

1. **Runtime behavior**: implement in Rust with Bouldy. Use Bouldy for init/tick/shutdown, logging, reflection-safe access, and narrow hook boundaries.
2. **Native loading**: install through UE4SS-compatible mod layout. UE4SS C++ mods are loaded from `Mods/<ModName>/dlls/main.dll` and enabled through `mods.txt` or `enabled.txt`; prefer `mods.txt` when load order matters.
3. **Game discovery**: use UE4SS development builds for live object/property inspection, SDK/header dumps, UHT-compatible headers, and `.usmap` generation when needed. Do not commit generated dumps unless the repo policy permits it.
4. **Mirror project**: for Blueprint or cooked asset mods, create a minimal Unreal project matching the game's UE major/minor version and plugin set as closely as practical. Generated UHT-compatible headers can bootstrap compile-time stubs, but expect manual fixes for missing virtuals, delegates, bad property flags, constructor signatures, and plugin mismatches.
5. **Cooked content**: keep content mods as UE plugin/content projects and cook them with Unreal's normal cooker. Use Asset Manager rules or Primary Asset Labels to isolate mod assets into nonzero chunks; chunks produce separate platform-specific `.pak` files when packaging is configured for paks and chunk generation.
6. **Iteration speed**: start with a tiny smoke-test mod that logs at load and resolves `/Script/CoreUObject.Object`; only then add game-specific object lookup, hooks, or cooked content.

Never present a generated mirror project as authoritative source code. It is a compatibility scaffold for compiling/cooking mod content against a shipped game.

## Runtime vs Asset Mod Boundaries

Put these in the Rust crate:

- game runtime lifecycle, hook registration, console/log helpers, object lookup wrappers, version checks;
- safe wrappers around Bouldy/UE reflection calls;
- optional deployment helper metadata for where the built DLL should be copied.

Keep these outside the Rust crate:

- Unreal Editor projects, generated headers, cooked assets, `.pak`, `.utoc`, `.ucas`, `.usmap`, extracted assets, SDK dumps;
- scripts that call UnrealPak, IoStore tools, UE commandlets, or project generators;
- large game-specific ABI notes, unless distilled into small checked assertions in Rust.

For hybrid modkits, add docs and empty directories rather than fake assets or fake SDK data.

## Cargo Defaults

- Package name: `<game-slug>-bouldy-mod`.
- Library crate type: `cdylib`.
- Edition: match repo default, otherwise `2021`.
- Dependency: `bouldy-runtime`.
- Avoid extra dependencies until needed.

Example:

```toml
[package]
name = "example-game-bouldy-mod"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
bouldy-runtime = { git = "ssh://git@github.com/caniko/rs-bouldy.git" }
```

Use a path dependency instead when the crate is developed in a sibling checkout:

```toml
bouldy-runtime = { path = "../rs_bouldy/crates/bouldy-runtime" }
```

## Rust Entry Point Template

Start with a minimal runtime mod that proves load/init/tick/shutdown:

```rust
use bouldy_runtime::prelude::*;

#[unreal_mod]
#[derive(Default)]
struct GameMod {
    frames: u64,
}

impl Mod for GameMod {
    fn on_init(&mut self, ctx: &mut ModContext) {
        ctx.log("Game Bouldy mod initialized");
    }

    fn on_tick(&mut self, delta: f32) {
        self.frames = self.frames.saturating_add(1);
        let _ = delta;
    }

    fn on_shutdown(&mut self) {
        log("Game Bouldy mod shutting down");
    }
}
```

Only add game modules once there is concrete behavior to implement.

## Loader/UE4SS Notes

Document deployment with this shape:

```text
<Game>/Binaries/Win64/
  UE4SS.dll
  Mods/
    <ModName>/
      dlls/
        main.dll
```

Mention that `mods.txt` should contain `<ModName> : 1` before the built-in keybind section when ordering matters. If using an `enabled.txt` shortcut, call out that it is simpler but does not express load order.

When a tiny C++ shim is required, make it only responsible for loading the Rust DLL and forwarding UE4SS/Bouldy lifecycle calls. Do not duplicate game logic in the shim.

## Asset Modkit Notes

If the user asks for asset modkit support, create docs and placeholders that tell future agents how to proceed without pretending to have game data:

- `modkit/mirror-project.md`: game UE version, matching plugin assumptions, generated-header source, known compile fixes, and regeneration command placeholders.
- `modkit/asset-workflow.md`: content plugin layout, cook target platform, pak/chunk naming, deployment location, and verification checklist.
- `tools/README.md`: where local-only scripts may live; do not include proprietary SDKs, dumped headers, extracted content, encryption keys, or game assets.

For cooked content:

- use a UE content plugin or clearly isolated `/Game/Mods/<ModName>` folder;
- use Primary Asset Labels or Asset Manager rules to put mod assets in a nonzero chunk;
- verify chunk output in `Saved/StagedBuilds/<Platform>/<Project>/Content/Paks`;
- remember cooked outputs are platform-specific and must match the target game's engine/package settings;
- if the game uses Io Store, document that the output may be `.utoc`/`.ucas` plus `.pak`, and that runtime mounting support depends on the game's configuration.

For mirror projects:

- match the game's UE major/minor version before debugging generated-code failures;
- add required engine/game plugins explicitly;
- keep generated UHT headers and CXX/SDK dumps local unless the user explicitly wants them tracked and redistribution is allowed;
- expect UHT generation to need manual fixes, especially for older UE4 versions and unreflected virtual functions.

## README Contents

Each modkit README should include:

- Game name and supported game build/version, or "unknown/unverified" if not known.
- Unreal Engine version and packaging format if known.
- Loader assumption: UE4SS-compatible Bouldy shim.
- Build command for Windows DLL.
- DLL output path.
- Deployment sketch for placing the Rust DLL at `Mods/<ModName>/dlls/main.dll`.
- `mods.txt` enablement line.
- Clear separation between runtime DLL workflow and optional asset-mod workflow.
- Mirror-project and cooked-content assumptions if `modkit/` exists.
- Safety note for any game-specific pointers, offsets, hooks, or reflection use.
- Legal/online-play note: respect game terms, avoid anti-cheat/online use unless explicitly allowed, and do not redistribute extracted proprietary game content.

## Validation

Run the strongest available checks:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --target x86_64-pc-windows-gnu
```

With Nix:

```bash
nix develop -c cargo test --workspace
nix develop .#windows -c cargo build --release --target x86_64-pc-windows-gnu
```

If the repo is a fresh Git checkout with a flake, stage new flake-visible files before `nix build` or `nix develop`.

For modkit docs/workspace changes, also verify:

```bash
find . -maxdepth 3 -type f | sort
rg -n "TODO|PLACEHOLDER|unknown|unverified" README.md modkit loader tools src Cargo.toml
```

Placeholders are allowed only when they are explicit assumptions rather than hidden fake facts.

## Future Extensions

When the modkit grows, split optional functionality into local modules or crates:

- `reflection` for UObject/UClass/FName wrappers once backed by real ABI data.
- `hooks` for UE4SS/native hook registration.
- `game_version` for explicit build/version checks.
- `interop` for optional offline data generated outside runtime by tools such as `unreal_asset`.
- `asset_pipeline` or `tools/` for local-only project generation, chunk verification, and packaging helpers, kept outside the runtime crate.

## Research Anchors

These are the durable facts this skill should preserve:

- UE4SS provides Lua, Blueprint, and C++ modding APIs, live property inspection, UHT-compatible header generation, C++ header dumps with offsets, and `.usmap` mapping dumps for UE4/UE5 games: https://docs.ue4ss.com/
- UE4SS C++ mod installation convention is `Mods/<ModName>/dlls/main.dll`, enabled through `mods.txt`; `enabled.txt` is possible but loses load-order control: https://docs.ue4ss.com/dev/guides/installing-a-c%2B%2B-mod.html
- UE4SS UHT header generation can create mirror-project inputs, but generated projects often need manual fixes for virtuals, delegates, property flags, constructors, plugin manifests, and engine-version mismatches: https://docs.ue4ss.com/guides/generating-uht-compatible-headers.html
- Unreal chunking uses Asset Manager rules or Primary Asset Labels to assign assets to chunk IDs; nonzero chunks package as separate platform-specific pak outputs when packaging with chunks enabled: https://dev.epicgames.com/documentation/en-us/unreal-engine/cooking-content-and-creating-chunks-in-unreal-engine
- Unreal's chunk preparation flow requires `Use Pak File` and `Generate Chunks`, then cooking/packaging content for the target platform: https://dev.epicgames.com/documentation/en-us/unreal-engine/preparing-assets-for-chunking-in-unreal-engine
