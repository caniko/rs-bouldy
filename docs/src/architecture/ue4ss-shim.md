# UE4SS Shim

The C++ shim is responsible for connecting UE4SS lifecycle events to the Rust mod DLL.

At startup, it:

1. Loads the Rust DLL with `LoadLibraryW`.
2. Resolves `bouldy_rust_init_v1`.
3. Constructs an `UnrealApiV1` table.
4. Calls the Rust init function.

At runtime, it forwards UE4SS tick and shutdown events to the callbacks registered by Rust.

Dynamic loading avoids import-library compatibility issues between MSVC-built C++ code and MinGW-built Rust DLLs.

See `cpp-shim/bouldy_ue4ss_shim.cpp` in the source tree for the minimal example.

