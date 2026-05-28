use crate::protocol::{ControlError, ErrorCode, TargetSelector, WindowSelector};

pub fn validate_foundation_tab_create_target(target: &TargetSelector) -> Result<(), ControlError> {
    if !matches!(target.window, None | Some(WindowSelector::Active)) {
        return Err(ControlError::new(
            ErrorCode::InvalidSelector,
            "tab.create supports only the active window selector in the foundation slice",
        ));
    }
    if target.tab.is_some()
        || target.pane.is_some()
        || target.session.is_some()
        || target.block.is_some()
    {
        return Err(ControlError::new(
            ErrorCode::InvalidSelector,
            "tab.create does not support lower-level target selectors in the foundation slice",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{TabSelector, TargetSelector};

    #[test]
    fn tab_create_allows_default_and_active_window_only() {
        assert!(validate_foundation_tab_create_target(&TargetSelector::default()).is_ok());
        assert!(
            validate_foundation_tab_create_target(&TargetSelector {
                window: Some(WindowSelector::Active),
                ..Default::default()
            })
            .is_ok()
        );
        assert_eq!(
            validate_foundation_tab_create_target(&TargetSelector {
                tab: Some(TabSelector::Active),
                ..Default::default()
            })
            .unwrap_err()
            .code,
            ErrorCode::InvalidSelector
        );
    }
}
