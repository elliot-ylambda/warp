use ::local_control::protocol::{
    PaneSelector, PaneTarget, TabSelector, TabTarget, TargetSelector, WindowSelector, WindowTarget,
};

use super::{
    active_window_id, capabilities, outside_warp_action_enabled_for_settings,
    validate_tab_create_target,
};
use crate::settings::{
    AllowInsideWarpControl, AllowInsideWarpReadOnly, AllowInsideWarpReadWrite,
    AllowOutsideWarpControl, AllowOutsideWarpReadOnly, AllowOutsideWarpReadWrite,
    LocalControlSettings,
};
use ::local_control::protocol::ActionKind;
use ::local_control::ErrorCode;
use settings::Setting as _;
fn settings_with_outside_warp(
    outside_control: bool,
    outside_read_write: bool,
) -> LocalControlSettings {
    LocalControlSettings {
        allow_inside_warp_control: AllowInsideWarpControl::new(Some(true)),
        allow_outside_warp_control: AllowOutsideWarpControl::new(Some(outside_control)),
        allow_inside_warp_read_only: AllowInsideWarpReadOnly::new(Some(true)),
        allow_outside_warp_read_only: AllowOutsideWarpReadOnly::new(Some(false)),
        allow_inside_warp_read_write: AllowInsideWarpReadWrite::new(Some(true)),
        allow_outside_warp_read_write: AllowOutsideWarpReadWrite::new(Some(outside_read_write)),
    }
}

#[test]
fn tab_create_accepts_default_and_active_targets() {
    validate_tab_create_target(&TargetSelector::default()).expect("default target is accepted");

    validate_tab_create_target(&TargetSelector {
        window: Some(WindowTarget::Active),
        tab: Some(TabTarget::Active),
        pane: Some(PaneTarget::Active),
    })
    .expect("active target is accepted");
}

#[test]
fn tab_create_rejects_concrete_targets() {
    let err = validate_tab_create_target(&TargetSelector {
        window: Some(WindowTarget::Id {
            id: WindowSelector("window".to_owned()),
        }),
        tab: None,
        pane: None,
    })
    .expect_err("concrete window target is rejected");
    assert_eq!(err.code, ErrorCode::InvalidSelector);

    let err = validate_tab_create_target(&TargetSelector {
        window: None,
        tab: Some(TabTarget::Id {
            id: TabSelector("tab".to_owned()),
        }),
        pane: None,
    })
    .expect_err("concrete tab target is rejected");
    assert_eq!(err.code, ErrorCode::InvalidSelector);

    let err = validate_tab_create_target(&TargetSelector {
        window: None,
        tab: None,
        pane: Some(PaneTarget::Id {
            id: PaneSelector("pane".to_owned()),
        }),
    })
    .expect_err("concrete pane target is rejected");
    assert_eq!(err.code, ErrorCode::InvalidSelector);
}

#[test]
fn capabilities_advertises_only_first_slice_core_actions() {
    assert_eq!(
        capabilities(),
        vec![
            ActionKind::InstanceList,
            ActionKind::AppPing,
            ActionKind::AppVersion,
            ActionKind::TabCreate,
        ]
    );
}

#[test]
fn outside_warp_discovery_requires_context_and_action_permission() {
    assert!(!outside_warp_action_enabled_for_settings(
        &settings_with_outside_warp(false, true),
        ActionKind::TabCreate
    ));
    assert!(!outside_warp_action_enabled_for_settings(
        &settings_with_outside_warp(true, false),
        ActionKind::TabCreate
    ));
    assert!(outside_warp_action_enabled_for_settings(
        &settings_with_outside_warp(true, true),
        ActionKind::TabCreate
    ));
}

#[test]
fn tab_create_prefers_active_window() {
    let active = warpui::WindowId::from_usize(1);

    assert_eq!(active_window_id(Some(active)), Some(active));
}

#[test]
fn tab_create_rejects_missing_active_window() {
    assert_eq!(active_window_id(None), None);
}
