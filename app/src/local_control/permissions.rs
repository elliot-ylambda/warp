use chrono::{Duration, Utc};
use local_control::auth::{credential_request_allowed, ScopedCredential};
use local_control::catalog::metadata_for;
use local_control::protocol::{ControlActionKind, ErrorCode, InvocationContext};
use warpui::{AppContext, SingletonEntity as _};

use crate::settings::LocalControlSettings;

pub fn ensure_outside_warp_enabled(ctx: &AppContext) -> Result<(), ErrorCode> {
    let settings = LocalControlSettings::as_ref(ctx);
    if *settings.outside_warp_control_enabled {
        Ok(())
    } else {
        Err(ErrorCode::LocalControlDisabled)
    }
}

pub fn issue_credential(
    ctx: &AppContext,
    instance_id: String,
    action: ControlActionKind,
    context: InvocationContext,
) -> Result<ScopedCredential, ErrorCode> {
    credential_request_allowed(context)?;
    ensure_outside_warp_enabled(ctx)?;
    let metadata = metadata_for(action).ok_or(ErrorCode::UnsupportedAction)?;
    if !metadata.allowed_contexts.contains(&context) {
        return Err(ErrorCode::ExecutionContextNotAllowed);
    }
    let settings = LocalControlSettings::as_ref(ctx);
    if !settings.outside_warp_permission_enabled(metadata.permission_category) {
        return Err(ErrorCode::InsufficientPermissions);
    }
    Ok(ScopedCredential::issue(
        instance_id,
        action,
        context,
        vec![metadata.permission_category],
        Duration::seconds(30),
    ))
}

pub fn verify_request(
    ctx: &AppContext,
    credential: &ScopedCredential,
    action: ControlActionKind,
) -> Result<(), ErrorCode> {
    ensure_outside_warp_enabled(ctx)?;
    credential.verify(action, Utc::now())?;
    let metadata = metadata_for(action).ok_or(ErrorCode::UnsupportedAction)?;
    let settings = LocalControlSettings::as_ref(ctx);
    if !settings.outside_warp_permission_enabled(metadata.permission_category) {
        return Err(ErrorCode::InsufficientPermissions);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use settings::{PrivatePreferences, PublicPreferences, Setting as _, SettingsManager};
    use warpui::SingletonEntity as _;
    use warpui_extras::user_preferences;

    use super::*;

    fn init_test_app(ctx: &mut warpui::AppContext) {
        ctx.add_singleton_model(move |_| {
            PublicPreferences::new(
                Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
            )
        });
        ctx.add_singleton_model(move |_| -> PrivatePreferences {
            PrivatePreferences::new(
                Box::<user_preferences::in_memory::InMemoryPreferences>::default(),
            )
        });
        ctx.add_singleton_model(|_| SettingsManager::default());
        LocalControlSettings::register(ctx);
    }

    #[test]
    fn disabled_outside_warp_denies_credentials() {
        warpui::App::test((), |mut app| async move {
            app.update(init_test_app);
            app.read(|ctx| {
                assert_eq!(
                    issue_credential(
                        ctx,
                        "i".into(),
                        ControlActionKind::AppPing,
                        InvocationContext::OutsideWarp,
                    )
                    .unwrap_err(),
                    ErrorCode::LocalControlDisabled
                );
            });
        });
    }

    #[test]
    fn inside_warp_context_is_denied() {
        warpui::App::test((), |mut app| async move {
            app.update(init_test_app);
            app.update(|ctx| {
                LocalControlSettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings
                        .outside_warp_control_enabled
                        .set_value(true, ctx)
                        .unwrap();
                    settings
                        .outside_warp_metadata_reads_enabled
                        .set_value(true, ctx)
                        .unwrap();
                });
            });
            app.read(|ctx| {
                assert_eq!(
                    issue_credential(
                        ctx,
                        "i".into(),
                        ControlActionKind::AppPing,
                        InvocationContext::InsideWarp,
                    )
                    .unwrap_err(),
                    ErrorCode::ExecutionContextNotAllowed
                );
            });
        });
    }
}
