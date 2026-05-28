use crate::protocol::{ErrorCode, TargetSelector};

pub fn validate_tab_create_target(target: &TargetSelector) -> Result<(), ErrorCode> {
    if target.only_default_or_active_window() {
        Ok(())
    } else {
        Err(ErrorCode::InvalidSelector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{TargetSelector, WindowSelector};

    #[test]
    fn tab_create_accepts_only_default_active_window() {
        assert_eq!(
            validate_tab_create_target(&TargetSelector::default()),
            Ok(())
        );
        assert_eq!(
            validate_tab_create_target(&TargetSelector {
                window: Some(WindowSelector::Active),
                ..TargetSelector::default()
            }),
            Ok(())
        );
        assert_eq!(
            validate_tab_create_target(&TargetSelector {
                window: Some(WindowSelector::Index(1)),
                ..TargetSelector::default()
            }),
            Err(ErrorCode::InvalidSelector)
        );
    }
}
