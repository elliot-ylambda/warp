//! TEMPORARY STUB — replaced at integration by the view agent's real module.
//!
//! The editable cell-based Jupyter view is owned by a parallel agent and lives
//! at `crate::notebooks::file::jupyter`. Until both branches merge, this slice
//! (routing + hosting pane + open/save/conflict wiring) compiles against this
//! local stub, which reproduces the view agent's published public API exactly:
//!
//! ```ignore
//! pub fn new(content: &str, path: Option<LocalOrRemotePath>, ctx) -> Self;
//! pub fn set_content(&mut self, content: &str, ctx);
//! pub fn to_json(&self) -> String;
//! pub fn is_dirty(&self) -> bool;
//! pub fn mark_saved(&mut self, ctx);
//! pub fn request_save(&mut self, ctx);
//! pub fn path(&self) -> Option<&LocalOrRemotePath>;
//! pub fn title(&self) -> String;
//! pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration>;
//! ```
//!
//! plus `JupyterNotebookEvent` and a `BackingView` impl.
//!
//! INTEGRATION: delete this file and, in `jupyter_notebook_pane.rs`, replace the
//! stub import with:
//!   `use crate::notebooks::file::jupyter::{JupyterNotebookEvent, JupyterNotebookView};`
//! The real view's `PaneHeaderOverflowMenuAction` is `JupyterNotebookAction`;
//! this stub uses `()` since it is never wired into the responder chain.

use warp_util::local_or_remote_path::LocalOrRemotePath;
use warpui::elements::Empty;
use warpui::{AppContext, Element, Entity, ModelHandle, TypedActionView, View, ViewContext};

use crate::menu::MenuItem;
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::pane::{view, BackingView, PaneConfiguration, PaneEvent};

/// Events emitted by the cell-based notebook view (frozen interface — mirrors
/// the view agent's finalized enum).
#[derive(Debug, Clone)]
pub enum JupyterNotebookEvent {
    /// The notebook transitioned from clean to dirty.
    Dirtied,
    /// The user requested a save. `json` is the full `.ipynb` to persist.
    SaveRequested { json: String },
    /// The user switched to the raw view. `json` is the current notebook,
    /// including unsaved edits, so the raw editor shows them.
    RawRequested { json: String },
    /// The view gained focus.
    Focused,
    /// Pane-level events (Close / ToggleMaximized / FocusSelf), mirroring
    /// `FileNotebookView`.
    Pane(PaneEvent),
}

impl From<PaneEvent> for JupyterNotebookEvent {
    fn from(event: PaneEvent) -> Self {
        JupyterNotebookEvent::Pane(event)
    }
}

pub struct JupyterNotebookView {
    content: String,
    path: Option<LocalOrRemotePath>,
    dirty: bool,
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
}

impl JupyterNotebookView {
    pub fn new(
        content: &str,
        path: Option<LocalOrRemotePath>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let title = path
            .as_ref()
            .map(|p| p.display_path())
            .unwrap_or_else(|| "Untitled".to_string());
        let pane_configuration = ctx.add_model(|_ctx| PaneConfiguration::new(title));
        Self {
            content: content.to_string(),
            path,
            dirty: false,
            pane_configuration,
            focus_handle: None,
        }
    }

    /// Replace the notebook content from an external reload. Clears the dirty flag.
    pub fn set_content(&mut self, content: &str, ctx: &mut ViewContext<Self>) {
        self.content = content.to_string();
        self.dirty = false;
        ctx.notify();
    }

    /// The current `.ipynb` JSON (edits are synced eagerly in the real view).
    pub fn to_json(&self) -> String {
        self.content.clone()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Called by the host after a successful save.
    pub fn mark_saved(&mut self, ctx: &mut ViewContext<Self>) {
        self.dirty = false;
        ctx.notify();
    }

    /// Emit a [`JupyterNotebookEvent::SaveRequested`] with the current JSON.
    pub fn request_save(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(JupyterNotebookEvent::SaveRequested {
            json: self.to_json(),
        });
    }

    pub fn path(&self) -> Option<&LocalOrRemotePath> {
        self.path.as_ref()
    }

    pub fn title(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.display_path())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }
}

impl Entity for JupyterNotebookView {
    type Event = JupyterNotebookEvent;
}

impl View for JupyterNotebookView {
    fn ui_name() -> &'static str {
        "JupyterNotebookView"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl TypedActionView for JupyterNotebookView {
    type Action = ();

    fn handle_action(&mut self, _action: &Self::Action, _ctx: &mut ViewContext<Self>) {}
}

impl BackingView for JupyterNotebookView {
    type PaneHeaderOverflowMenuAction = ();
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        _action: &Self::PaneHeaderOverflowMenuAction,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(JupyterNotebookEvent::Pane(PaneEvent::Close));
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(JupyterNotebookEvent::Focused);
    }

    fn pane_header_overflow_menu_items(
        &self,
        _ctx: &AppContext,
    ) -> Vec<MenuItem<Self::PaneHeaderOverflowMenuAction>> {
        vec![]
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        app: &AppContext,
    ) -> view::HeaderContent {
        let title = self.pane_configuration.as_ref(app).title().to_owned();
        view::HeaderContent::Standard(view::StandardHeader {
            title,
            title_secondary: None,
            title_style: None,
            title_clip_config: warpui::text_layout::ClipConfig::start(),
            title_max_width: None,
            left_of_title: None,
            right_of_title: None,
            left_of_overflow: None,
            options: Default::default(),
        })
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
