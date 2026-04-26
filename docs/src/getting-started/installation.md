# Installation

## Prerequisites

- Nix with flakes enabled, or a stable Rust toolchain with Cargo.
- For Windows DLL builds, the `x86_64-pc-windows-gnu` Rust target.
- A UE4SS mod/plugin project that can load a native DLL and forward lifecycle events.

## Clone

```bash
git clone ssh://git@codeberg.org/caniko/rs-bouldy.git
cd rs-bouldy
```

## Nix Development Shell

```bash
nix develop
```

The shell provides the Rust toolchain configured through `rs-harbor`.

## Fresh Git Repository Note

Nix flakes only see files tracked by Git. If you are working from a freshly initialized checkout with new files, stage them before running `nix develop` or `nix build`.

```bash
git add flake.nix flake.lock Cargo.toml Cargo.lock crates example-mod cpp-shim docs website README.md rust-toolchain.toml .gitignore
```

