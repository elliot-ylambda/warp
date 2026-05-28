pub mod bridge;
pub mod handlers;
pub mod permissions;
pub mod resolver;

#[cfg(not(target_family = "wasm"))]
mod server;

#[cfg(not(target_family = "wasm"))]
pub fn register(ctx: &mut warpui::AppContext) {
    use warp_core::features::FeatureFlag;
    if !FeatureFlag::WarpControlCli.is_enabled() {
        return;
    }

    let bridge = ctx.add_singleton_model(bridge::LocalControlBridge::new);
    let spawner = bridge.update(ctx, |_, ctx| ctx.spawner());
    ctx.add_singleton_model(move |ctx| server::LocalControlServer::new(spawner, ctx));
}
