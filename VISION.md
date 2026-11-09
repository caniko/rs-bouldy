# Bouldy vision

Bouldy is a small Rust workspace for runtime modding of Unreal Engine games: a
tiny C ABI between a loader shim and Rust mods, a safe `Mod` trait, a
`#[unreal_mod]` proc macro that generates the exported entrypoints, and a
minimal UE4SS-compatible C++ bridge to prove the loop end to end. It is for
people who want to write Unreal mods in Rust rather than C++, without adopting
a whole reverse-engineering toolkit.

## Problem

Unreal modding in Rust means hand-writing `extern "C"` entrypoints, managing a
global runtime, and remembering that every call crosses an FFI boundary into
C++ that may not survive a panic. Bouldy puts that boundary in one place:
`bouldy-sys` holds the raw types, `bouldy-runtime` holds the safe surface and
lifecycle, and mod authors implement `on_init`, `on_tick` and `on_shutdown`
without touching unsafe code.

## Principles

- **Deliberately narrow.** The MVP does not parse assets, build `.pak` files,
  generate Unreal SDK bindings, or contain game-specific logic. Offline tooling
  belongs to projects such as `AstroTechies/unrealmodding`; Bouldy stays on the
  runtime side.
- **One unsafe surface.** The C ABI boundary lives in `bouldy-sys` and
  `bouldy-runtime`. Mod authors only see the safe `Mod` trait and `ModContext`.
- **Panics never unwind into C++.** Every exported entrypoint and registered
  callback is wrapped in `catch_unwind`; a panic is logged when possible.
- **The ABI grows by versioned extension, not mutation.** `UnrealApiV1` embeds
  the base table and adds optional lifecycle registration; `UnrealApiV2` keeps
  `UnrealApiV1` as its first field and adds a separate discovery interface.
- **Discovery stays reconnaissance.** The discovery API scans broad Unreal
  concepts — objects, classes, functions, properties — for candidate symbols.
  It ships no SDK bindings, offsets, generated headers or patching behaviour.
- **The unsafe lint set is a gate.** `unsafe_op_in_unsafe_fn`,
  `undocumented_unsafe_blocks` and `missing_docs` are enforced at workspace
  level rather than left to review.

## Scope

In scope: raw C ABI types and bridge helpers; safe lifecycle, logging and the
mod trait; macro-generated entrypoints; an example `cdylib` mod; the C++ shim
example; the mdBook covering workspace layout, runtime ABI and safety model; and
Nix/Cargo workflows for native validation plus Windows DLL builds through a
`.#windows` dev shell.

## Non-goals

- Not a Unreal SDK generator, asset tool, or `.pak` builder.
- Not a game-specific mod. No game logic, offsets or patched classes live here;
  a mod that does something useful is written outside this workspace.
- Not a UE4SS replacement. The shim is an example bridge, not a loader.

## Direction

There is no roadmap document. Recent history is environment and CI upkeep:
Harbor/GitHub input migration, an `rs-harbor` toolchain profile, Crow/Simit
workflows, and refreshed generated skill views. The ABI already carries V1 and
V2 tables, so the visible direction is additive ABI revisions as new runtime
services are needed.

## Place in the fleet

Canix lists Bouldy as a Simit updater target and carries its skill catalogue
entry, so its flake and generated CI are refreshed fleet-wide. No Canix host
depends on it.
