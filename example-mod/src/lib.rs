//! Example Bouldy runtime mod.

use bouldy_runtime::prelude::*;

#[unreal_mod]
#[derive(Default)]
struct MyMod;

impl Mod for MyMod {
    fn on_init(&mut self, ctx: &mut ModContext) {
        ctx.log("Rust mod initialized");
    }

    fn on_tick(&mut self, delta: f32) {
        let _ = delta;
    }

    fn on_shutdown(&mut self) {
        log("Rust mod shutting down");
    }
}
