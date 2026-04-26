// Minimal Bouldy UE4SS bridge example.
//
// This file is intentionally small and self-contained. Adapt Startup, OnTick,
// and Shutdown to the lifecycle hooks exposed by your UE4SS mod template.

#include <windows.h>

#include <cstdint>
#include <cstdio>

extern "C" {
struct UnrealApi {
    void (*log)(const char* message);
    float (*get_delta_seconds)();
};

using TickCallback = void (*)(float delta_seconds);
using ShutdownCallback = void (*)();

struct UnrealApiV1 {
    UnrealApi base;
    void (*register_tick)(TickCallback callback);
    void (*register_shutdown)(ShutdownCallback callback);
};

using BouldyInitV1 = bool (*)(UnrealApiV1* api);
}

namespace {
HMODULE g_rust_module = nullptr;
TickCallback g_tick_callback = nullptr;
ShutdownCallback g_shutdown_callback = nullptr;
float g_last_delta_seconds = 0.0f;

void ShimLog(const char* message) {
    // Replace this with UE4SS's logging facility in a real shim.
    std::printf("[Bouldy] %s\n", message ? message : "<null>");
}

float GetDeltaSeconds() {
    return g_last_delta_seconds;
}

void RegisterTick(TickCallback callback) {
    g_tick_callback = callback;
}

void RegisterShutdown(ShutdownCallback callback) {
    g_shutdown_callback = callback;
}
} // namespace

bool Startup() {
    g_rust_module = LoadLibraryW(L"example_mod.dll");
    if (!g_rust_module) {
        ShimLog("failed to load example_mod.dll");
        return false;
    }

    auto init = reinterpret_cast<BouldyInitV1>(
        GetProcAddress(g_rust_module, "bouldy_rust_init_v1"));
    if (!init) {
        ShimLog("failed to find bouldy_rust_init_v1");
        FreeLibrary(g_rust_module);
        g_rust_module = nullptr;
        return false;
    }

    UnrealApiV1 api{};
    api.base.log = &ShimLog;
    api.base.get_delta_seconds = &GetDeltaSeconds;
    api.register_tick = &RegisterTick;
    api.register_shutdown = &RegisterShutdown;

    if (!init(&api)) {
        ShimLog("Rust mod initialization returned false");
        FreeLibrary(g_rust_module);
        g_rust_module = nullptr;
        return false;
    }

    return true;
}

void OnTick(float delta_seconds) {
    g_last_delta_seconds = delta_seconds;
    if (g_tick_callback) {
        g_tick_callback(delta_seconds);
    }
}

void Shutdown() {
    if (g_shutdown_callback) {
        g_shutdown_callback();
        g_shutdown_callback = nullptr;
    }

    g_tick_callback = nullptr;

    if (g_rust_module) {
        FreeLibrary(g_rust_module);
        g_rust_module = nullptr;
    }
}

