// Minimal Bouldy UE4SS bridge example.
//
// This file is intentionally small and self-contained. Adapt Startup, OnTick,
// Shutdown, and discovery callbacks to the lifecycle hooks exposed by your
// UE4SS mod template.

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

using DiscoveryVisitor = bool (*)(const void* candidate, void* user_data);

struct DiscoveryFilter {
    const char* const* terms;
    size_t term_count;
    uint32_t kind_mask;
    size_t max_results;
};

struct DiscoveryCandidate {
    uint32_t kind;
    const char* name;
    const char* path;
    const char* owner;
    uint64_t flags;
};

struct DiscoveryCandidateV2 {
    uint32_t kind;
    uint32_t schema_version;
    const char* name;
    const char* path;
    const char* owner;
    uint64_t flags;
    int32_t chunk_index;
    int32_t object_index;
};

using DiscoveryVisitorV2 = bool (*)(const DiscoveryCandidateV2* candidate, void* user_data);

struct UnrealDiscoveryApiV2 {
    size_t (*scan)(const DiscoveryFilter* filter, DiscoveryVisitorV2 visitor, void* user_data);
    void (*export_record)(const char* channel, const char* payload);
};

struct UnrealApiV3 {
    UnrealApiV1 lifecycle;
    UnrealDiscoveryApiV2 discovery;
};

using BouldyInitV3 = bool (*)(UnrealApiV3* api);
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

size_t ScanDiscovery(const DiscoveryFilter* filter, DiscoveryVisitorV2 visitor, void* user_data) {
    // Replace this stub with UE4SS-backed object/class/function/property
    // enumeration. The filter terms are generic UTF-8 strings supplied by Rust.
    if (!filter || !visitor) {
        return 0;
    }

    ShimLog("Bouldy discovery scan requested");
    for (size_t index = 0; index < filter->term_count; ++index) {
        ShimLog(filter->terms[index] ? filter->terms[index] : "<null term>");
    }

    return 0;
}

void ExportDiscoveryRecord(const char* channel, const char* payload) {
    // Replace this with file export or UE4SS logging in a real shim.
    std::printf("[Bouldy:%s] %s\n", channel ? channel : "discovery",
                payload ? payload : "<null>");
}
} // namespace

bool Startup() {
    g_rust_module = LoadLibraryW(L"example_mod.dll");
    if (!g_rust_module) {
        ShimLog("failed to load example_mod.dll");
        return false;
    }

    auto init = reinterpret_cast<BouldyInitV3>(
        GetProcAddress(g_rust_module, "bouldy_rust_init_v3"));
    if (!init) {
        ShimLog("failed to find bouldy_rust_init_v3");
        FreeLibrary(g_rust_module);
        g_rust_module = nullptr;
        return false;
    }

    UnrealApiV3 api{};
    api.lifecycle.base.log = &ShimLog;
    api.lifecycle.base.get_delta_seconds = &GetDeltaSeconds;
    api.lifecycle.register_tick = &RegisterTick;
    api.lifecycle.register_shutdown = &RegisterShutdown;
    api.discovery.scan = &ScanDiscovery;
    api.discovery.export_record = &ExportDiscoveryRecord;

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
