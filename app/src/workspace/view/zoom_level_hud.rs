use std::time::Duration;

use crate::appearance::Appearance;
use crate::ui_components::blended_colors;
use crate::window_settings::ZoomLevel;
use warp_core::ui::theme::Fill;
use warpui::elements::{
    Border, Container, CornerRadius, Element, Empty, FormattedTextElement, Radius,
};
use warpui::fonts::Weight;
use warpui::r#async::{SpawnedFutureHandle, Timer};
use warpui::{AppContext, Entity, SingletonEntity, View, ViewContext};

const ZOOM_LEVEL_HUD_TIMEOUT: Duration = Duration::from_millis(1000);

pub struct ZoomLevelHud {
    visible_zoom_level: Option<u16>,
    dismiss_handle: Option<SpawnedFutureHandle>,
}

impl ZoomLevelHud {
    pub fn new() -> Self {
        Self {
            visible_zoom_level: None,
            dismiss_handle: None,
        }
    }

    pub fn show_zoom_level(&mut self, zoom_level: u16, ctx: &mut ViewContext<Self>) {
        self.visible_zoom_level = Some(zoom_level);
        self.restart_dismiss_timer(ctx);
        ctx.notify();
    }

    fn restart_dismiss_timer(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(handle) = self.dismiss_handle.take() {
            handle.abort();
        }

        let handle = ctx.spawn_abortable(
            Timer::after(ZOOM_LEVEL_HUD_TIMEOUT),
            |view, _, ctx| {
                view.visible_zoom_level = None;
                view.dismiss_handle = None;
                ctx.notify();
            },
            |_, _| {},
        );
        self.dismiss_handle = Some(handle);
    }
}

impl Entity for ZoomLevelHud {
    type Event = ();
}

impl View for ZoomLevelHud {
    fn ui_name() -> &'static str {
        "ZoomLevelHud"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let Some(zoom_level) = self.visible_zoom_level else {
            return Empty::new().finish();
        };

        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let text = FormattedTextElement::from_str(
            format!("{zoom_level}%"),
            appearance.ui_font_family(),
            13.,
        )
        .with_color(blended_colors::text_main(
            theme,
            blended_colors::neutral_3(theme),
        ))
        .with_weight(Weight::Bold)
        .finish();

        Container::new(text)
            .with_horizontal_padding(12.)
            .with_vertical_padding(8.)
            .with_background(Fill::Solid(blended_colors::neutral_3(theme)).with_opacity(95))
            .with_border(Border::all(1.).with_border_color(blended_colors::neutral_5(theme)))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
            .finish()
    }
}

pub fn adjusted_zoom_level(current_zoom: u16, increase: bool) -> Option<u16> {
    let current_index = ZoomLevel::VALUES
        .iter()
        .position(|zoom| *zoom == current_zoom)?;

    let next_index = if increase {
        (current_index + 1).min(ZoomLevel::VALUES.len() - 1)
    } else {
        current_index.saturating_sub(1)
    };

    Some(ZoomLevel::VALUES[next_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjusted_zoom_level_steps_up() {
        assert_eq!(adjusted_zoom_level(100, true), Some(110));
        assert_eq!(adjusted_zoom_level(110, true), Some(125));
    }

    #[test]
    fn adjusted_zoom_level_steps_down() {
        assert_eq!(adjusted_zoom_level(100, false), Some(90));
        assert_eq!(adjusted_zoom_level(125, false), Some(110));
    }

    #[test]
    fn adjusted_zoom_level_clamps_at_bounds() {
        assert_eq!(adjusted_zoom_level(350, true), Some(350));
        assert_eq!(adjusted_zoom_level(50, false), Some(50));
    }

    #[test]
    fn adjusted_zoom_level_ignores_invalid_current_zoom() {
        assert_eq!(adjusted_zoom_level(95, true), None);
        assert_eq!(adjusted_zoom_level(95, false), None);
    }
}
