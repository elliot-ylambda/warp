use settings::{Setting as _, ToggleableSetting as _};
use warp_core::features::FeatureFlag;
use warpui::elements::Element;
use warpui::ui_components::components::UiComponent as _;
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle};

use super::settings_page::{
    render_body_item, LocalOnlyIconState, MatchData, PageType, SettingsPageMeta,
    SettingsPageViewHandle, SettingsWidget, ToggleState,
};
use super::SettingsSection;
use crate::appearance::Appearance;
use crate::report_if_error;
use crate::settings::{LocalControlPermissionSettings, LocalControlSettings};

#[derive(Clone, Debug)]
pub enum ScriptingPageAction {
    OutsideWarpControl,
    MetadataRead,
    UnderlyingDataRead,
    AppStateMutation,
    MetadataConfigurationMutation,
    UnderlyingDataMutation,
}

pub struct ScriptingPageView {
    page: PageType<Self>,
}

impl ScriptingPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&LocalControlSettings::handle(ctx), |_, _, _, ctx| {
            ctx.notify();
        });

        let page = PageType::new_uncategorized(
            vec![
                Box::new(OutsideWarpControlWidget::default()),
                Box::new(PermissionWidget::new(
                    "Read app metadata",
                    "Allow commands like `warpctrl app ping` and `warpctrl app version` to read non-sensitive app metadata.",
                    "local control metadata read permission app ping version",
                    ScriptingPageAction::MetadataRead,
                    |settings| settings.metadata_read,
                )),
                Box::new(PermissionWidget::new(
                    "Read underlying data",
                    "Reserved for future commands that read terminal, pane, or session data. Disabled by default.",
                    "local control underlying data read permission",
                    ScriptingPageAction::UnderlyingDataRead,
                    |settings| settings.underlying_data_read,
                )),
                Box::new(PermissionWidget::new(
                    "Mutate app state",
                    "Allow commands like `warpctrl tab create` to change local app state.",
                    "local control app state mutation tab create permission",
                    ScriptingPageAction::AppStateMutation,
                    |settings| settings.app_state_mutation,
                )),
                Box::new(PermissionWidget::new(
                    "Mutate metadata configuration",
                    "Reserved for future commands that update metadata configuration. Disabled by default.",
                    "local control metadata configuration mutation permission",
                    ScriptingPageAction::MetadataConfigurationMutation,
                    |settings| settings.metadata_configuration_mutation,
                )),
                Box::new(PermissionWidget::new(
                    "Mutate underlying data",
                    "Reserved for future commands that write terminal, pane, or session data. Disabled by default.",
                    "local control underlying data mutation permission",
                    ScriptingPageAction::UnderlyingDataMutation,
                    |settings| settings.underlying_data_mutation,
                )),
            ],
            Some("Scripting"),
        );

        Self { page }
    }

    fn update_permissions(
        &mut self,
        ctx: &mut ViewContext<Self>,
        update: impl FnOnce(&mut LocalControlPermissionSettings),
    ) {
        LocalControlSettings::handle(ctx).update(ctx, |settings, ctx| {
            let mut permissions = *settings.outside_warp_permissions.value();
            update(&mut permissions);
            report_if_error!(settings
                .outside_warp_permissions
                .set_value(permissions, ctx));
        });
        ctx.notify();
    }
}

impl Entity for ScriptingPageView {
    type Event = ();
}

impl TypedActionView for ScriptingPageView {
    type Action = ScriptingPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ScriptingPageAction::OutsideWarpControl => {
                LocalControlSettings::handle(ctx).update(ctx, |settings, ctx| {
                    report_if_error!(settings
                        .outside_warp_control_enabled
                        .toggle_and_save_value(ctx));
                });
                ctx.notify();
            }
            ScriptingPageAction::MetadataRead => {
                self.update_permissions(ctx, |settings| {
                    settings.metadata_read = !settings.metadata_read;
                });
            }
            ScriptingPageAction::UnderlyingDataRead => {
                self.update_permissions(ctx, |settings| {
                    settings.underlying_data_read = !settings.underlying_data_read;
                });
            }
            ScriptingPageAction::AppStateMutation => {
                self.update_permissions(ctx, |settings| {
                    settings.app_state_mutation = !settings.app_state_mutation;
                });
            }
            ScriptingPageAction::MetadataConfigurationMutation => {
                self.update_permissions(ctx, |settings| {
                    settings.metadata_configuration_mutation =
                        !settings.metadata_configuration_mutation;
                });
            }
            ScriptingPageAction::UnderlyingDataMutation => {
                self.update_permissions(ctx, |settings| {
                    settings.underlying_data_mutation = !settings.underlying_data_mutation;
                });
            }
        }
    }
}

impl View for ScriptingPageView {
    fn ui_name() -> &'static str {
        "ScriptingPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl SettingsPageMeta for ScriptingPageView {
    fn section() -> SettingsSection {
        SettingsSection::Scripting
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        FeatureFlag::WarpControlCli.is_enabled()
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<ScriptingPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<ScriptingPageView>) -> Self {
        SettingsPageViewHandle::Scripting(view_handle)
    }
}

#[derive(Default)]
struct OutsideWarpControlWidget {
    switch_state: SwitchStateHandle,
}

impl SettingsWidget for OutsideWarpControlWidget {
    type View = ScriptingPageView;

    fn search_terms(&self) -> &str {
        "local control warpctrl scripting outside warp"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let settings = LocalControlSettings::as_ref(app);
        render_body_item::<ScriptingPageAction>(
            "Enable outside-Warp local control".to_owned(),
            None,
            LocalOnlyIconState::Hidden,
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(*settings.outside_warp_control_enabled.value())
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(ScriptingPageAction::OutsideWarpControl);
                })
                .finish(),
            Some(
                "Allows external `warpctrl` processes on this machine to request short-lived scoped credentials. Disabled by default."
                    .to_owned(),
            ),
        )
    }
}

struct PermissionWidget {
    title: &'static str,
    description: &'static str,
    search_terms: &'static str,
    action: ScriptingPageAction,
    value: fn(&LocalControlPermissionSettings) -> bool,
    switch_state: SwitchStateHandle,
}

impl PermissionWidget {
    fn new(
        title: &'static str,
        description: &'static str,
        search_terms: &'static str,
        action: ScriptingPageAction,
        value: fn(&LocalControlPermissionSettings) -> bool,
    ) -> Self {
        Self {
            title,
            description,
            search_terms,
            action,
            value,
            switch_state: SwitchStateHandle::default(),
        }
    }
}

impl SettingsWidget for PermissionWidget {
    type View = ScriptingPageView;

    fn search_terms(&self) -> &str {
        self.search_terms
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let settings = LocalControlSettings::as_ref(app);
        let outside_warp_enabled = *settings.outside_warp_control_enabled.value();
        let is_checked = (self.value)(settings.outside_warp_permissions.value());
        let toggle_state = ToggleState::from(outside_warp_enabled);
        let action = self.action.clone();
        let switch = appearance
            .ui_builder()
            .switch(self.switch_state.clone())
            .check(is_checked);
        let switch = if outside_warp_enabled {
            switch
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(action.clone());
                })
                .finish()
        } else {
            switch.disable().build().finish()
        };

        render_body_item::<ScriptingPageAction>(
            self.title.to_owned(),
            None,
            LocalOnlyIconState::Hidden,
            toggle_state,
            appearance,
            switch,
            Some(self.description.to_owned()),
        )
    }
}
