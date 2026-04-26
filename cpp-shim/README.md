# Bouldy UE4SS Shim

This directory contains a minimal C++ bridge showing how UE4SS can load a Rust Bouldy mod DLL, forward lifecycle events, and expose a generic discovery backend.

The sample intentionally uses `LoadLibraryW`/`GetProcAddress` instead of linking against a Rust import library. That keeps the shim usable when Rust is built with MinGW while UE4SS and the game are built with MSVC.

Expected deployment shape:

```text
Game/Binaries/Win64/
  UE4SS/
    Mods/
      BouldyShim/
        bouldy_ue4ss_shim.dll
        example_mod.dll
```

The exact UE4SS plugin/module boilerplate depends on the UE4SS version and mod template. Wire the `Startup`, `OnTick`, and `Shutdown` functions from `bouldy_ue4ss_shim.cpp` into the equivalent UE4SS lifecycle callbacks.

The sample `ScanDiscovery` function is a stub. A real game shim should fill it with UE4SS-backed enumeration of reflected objects, classes, functions, and properties, then pass only generic `DiscoveryCandidate` records back to Rust. Keep game-specific symbols, offsets, and hook decisions out of Bouldy core.
