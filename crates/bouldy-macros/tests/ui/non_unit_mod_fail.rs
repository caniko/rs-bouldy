extern crate self as bouldy_runtime;

use bouldy_macros::unreal_mod;

pub type TickCallback = extern "C" fn(f32);
pub type ShutdownCallback = extern "C" fn();

pub struct UnrealApi;
pub struct UnrealApiV1;
pub struct ModContext;

pub trait Mod {
    fn on_init(&mut self, _ctx: &mut ModContext) {}
    fn on_tick(&mut self, _delta: f32) {}
    fn on_shutdown(&mut self) {}
}

pub fn init_with_base_api(_api: *mut UnrealApi, init: impl FnOnce(&mut ModContext)) -> bool {
    let mut ctx = ModContext;
    init(&mut ctx);
    true
}

pub fn init_with_v1_api(
    _api: *mut UnrealApiV1,
    _tick: TickCallback,
    _shutdown: ShutdownCallback,
    init: impl FnOnce(&mut ModContext),
) -> bool {
    let mut ctx = ModContext;
    init(&mut ctx);
    true
}

pub fn tick_registered_mod(delta: f32, tick: impl FnOnce(f32)) {
    tick(delta);
}

pub fn shutdown_registered_mod(shutdown: impl FnOnce()) {
    shutdown();
}

#[unreal_mod]
#[derive(Default)]
struct BadMod {
    field: u32,
}

impl Mod for BadMod {}

fn main() {}

