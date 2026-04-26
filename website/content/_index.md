+++
title = "Bouldy"

[extra]
tagline = "Rust runtime modding for Unreal Engine"
subtitle = "A minimal open-source workspace for writing Rust mods that run inside UE4 and UE5 games through a small UE4SS-compatible shim."
quick_install = "cargo build -p example-mod --release --target x86_64-pc-windows-gnu"

[[extra.features]]
title = "Runtime First"
description = "Focuses on in-memory lifecycle and logic: init, tick, shutdown, and host-provided runtime services."

[[extra.features]]
title = "Safe Mod API"
description = "Mod authors implement a small Rust trait while raw pointers and FFI calls stay in runtime layers."

[[extra.features]]
title = "UE4SS Compatible"
description = "A small C++ shim loads the Rust DLL, provides logging and delta time, and forwards lifecycle callbacks."

[[extra.features]]
title = "Versioned ABI"
description = "Keeps the base C API tiny while allowing optional lifecycle registration through UnrealApiV1."

[[extra.features]]
title = "Panic Containment"
description = "C ABI entrypoints and callbacks catch panics so Rust unwinding does not cross into C++."

[[extra.features]]
title = "Complementary Scope"
description = "Leaves asset parsing, pak building, and offline pipelines to dedicated Unreal tooling."
+++

