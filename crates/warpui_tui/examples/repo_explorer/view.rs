//! The `repo_explorer` root view and the bootstrap that wires the real
//! `repo_metadata` model into the shared core.
//!
//! This module is compiled both by the example binary (`main.rs`) and by the
//! headless integration test (`tests/repo_explorer_integration.rs`, via a
//! `#[path]` include), so the test drives the *exact* same view + model wiring
//! the interactive program does — no forked logic.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::style::Color;
use repo_metadata::local_model::{GetContentsArgs, IndexedRepoState, RepoContent};
use repo_metadata::repositories::DetectedRepositories;
use repo_metadata::watcher::DirectoryWatcher;
use repo_metadata::{RepoMetadataModel, RepositoryIdentifier};
use warp_util::standardized_path::StandardizedPath;
use warpui_core::platform::WindowStyle;
use warpui_core::{
    AddWindowOptions, App, AppContext, Entity, ModelHandle, TuiTypedActionView, TuiView,
    TuiViewContext, TuiViewHandle, WindowId,
};
use warpui_tui::{TuiColumn, TuiElement, TuiEventHandler, TuiRenderOutput, TuiStyle, TuiText};

/// The typed action the view handles, dispatched through the shared core.
#[derive(Debug, Clone, Copy)]
pub enum NavAction {
    SelectNext,
    SelectPrev,
}

/// The root TUI view. Holds a handle to the real model and the current selection.
pub struct RepoExplorerView {
    model: ModelHandle<RepoMetadataModel>,
    repo_id: RepositoryIdentifier,
    repo_root: StandardizedPath,
    selected: usize,
    quit: Rc<Cell<bool>>,
}

impl RepoExplorerView {
    /// Collects the (display name, is_dir) of the repository's indexed entries
    /// straight from the model, excluding the repository root itself. The
    /// borrowed `RepoContents` is dropped before the owned names are returned.
    ///
    /// The model's traversal order is store-defined (and not stable), so the
    /// list is sorted here — directories first, then alphabetically — to give a
    /// deterministic, navigable view.
    pub fn entries(&self, ctx: &AppContext) -> Vec<(String, bool)> {
        let model = self.model.as_ref(ctx);
        let Ok(contents) = model.get_repo_contents(&self.repo_id, GetContentsArgs::default(), ctx)
        else {
            return Vec::new();
        };
        let mut entries: Vec<(String, bool)> = contents
            .contents
            .iter()
            .filter_map(|entry| {
                let (path, is_dir) = match entry {
                    RepoContent::File(file) => (file.path.as_ref(), false),
                    RepoContent::Directory(dir) => (dir.path.as_ref(), true),
                };
                // Skip the repository root itself; it is already in the header.
                if path == &self.repo_root {
                    return None;
                }
                Some((display_name(path), is_dir))
            })
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        entries
    }

    /// The currently selected entry index. Used by the integration test to
    /// assert that a dispatched typed action advanced the selection.
    #[allow(dead_code)]
    pub fn selected(&self) -> usize {
        self.selected
    }

    fn status_line(&self, ctx: &AppContext, entry_count: usize) -> String {
        let model = self.model.as_ref(ctx);
        match model.repository_state(&self.repo_id, ctx) {
            Some(IndexedRepoState::Indexed(_)) => {
                format!("status: indexed · {entry_count} entries")
            }
            Some(IndexedRepoState::Pending(_)) => "status: indexing…".to_owned(),
            Some(IndexedRepoState::Failed(error)) => format!("status: failed: {error}"),
            None => "status: not tracked".to_owned(),
        }
    }
}

impl Entity for RepoExplorerView {
    type Event = ();
}

impl TuiView for RepoExplorerView {
    type RenderOutput = TuiRenderOutput;

    fn ui_name() -> &'static str {
        "RepoExplorerView"
    }

    fn render_tui(&self, ctx: &AppContext) -> TuiRenderOutput {
        let entries = self.entries(ctx);
        let selected = self.selected.min(entries.len().saturating_sub(1));

        let header_style = TuiStyle::default()
            .with_bold(true)
            .with_foreground(Color::Cyan);
        let dir_style = TuiStyle::default().with_foreground(Color::Blue);
        let selected_style = TuiStyle::default().with_reversed(true).with_bold(true);
        let hint_style = TuiStyle::default().with_dim(true);

        let mut rows: Vec<Box<dyn TuiElement>> = Vec::new();
        rows.push(Box::new(
            TuiText::new(format!("repo_metadata · {}", display_name(&self.repo_root)))
                .with_style(header_style)
                .truncate(),
        ));
        rows.push(Box::new(
            TuiText::new(self.status_line(ctx, entries.len())).truncate(),
        ));
        rows.push(Box::new(TuiText::new(" ")));

        if entries.is_empty() {
            rows.push(Box::new(
                TuiText::new("(no indexed entries yet)").with_style(hint_style),
            ));
        }
        for (index, (name, is_dir)) in entries.iter().enumerate() {
            let marker = if index == selected { "› " } else { "  " };
            let suffix = if *is_dir { "/" } else { "" };
            let style = if index == selected {
                selected_style
            } else if *is_dir {
                dir_style
            } else {
                TuiStyle::default()
            };
            rows.push(Box::new(
                TuiText::new(format!("{marker}{name}{suffix}"))
                    .with_style(style)
                    .truncate(),
            ));
        }

        rows.push(Box::new(TuiText::new(" ")));
        rows.push(Box::new(
            TuiText::new("j/↓ next · k/↑ prev · q quit")
                .with_style(hint_style)
                .truncate(),
        ));

        let body = TuiColumn::with_children(rows);

        // Wire keyboard input: navigation keys dispatch a typed action through
        // the shared core; quit keys flip the shared quit flag the runtime polls.
        let quit_for_q = self.quit.clone();
        let quit_for_esc = self.quit.clone();
        let handler = TuiEventHandler::new(body)
            .on_key("j", |_, ctx, _| {
                ctx.dispatch_typed_action(NavAction::SelectNext)
            })
            .on_key("down", |_, ctx, _| {
                ctx.dispatch_typed_action(NavAction::SelectNext)
            })
            .on_key("k", |_, ctx, _| {
                ctx.dispatch_typed_action(NavAction::SelectPrev)
            })
            .on_key("up", |_, ctx, _| {
                ctx.dispatch_typed_action(NavAction::SelectPrev)
            })
            .on_key("q", move |_, _, _| quit_for_q.set(true))
            .on_key("escape", move |_, _, _| quit_for_esc.set(true));

        Box::new(handler)
    }
}

impl TuiTypedActionView for RepoExplorerView {
    type Action = NavAction;

    fn handle_action(&mut self, action: &NavAction, ctx: &mut TuiViewContext<Self>) {
        let count = self.entries(ctx).len();
        if count == 0 {
            return;
        }
        match action {
            NavAction::SelectNext => {
                self.selected = (self.selected + 1).min(count - 1);
            }
            NavAction::SelectPrev => {
                self.selected = self.selected.saturating_sub(1);
            }
        }
        // Mark the view dirty so the runtime repaints with the new selection.
        ctx.notify();
    }
}

/// The display name for a repository entry: its final path component, falling
/// back to the full path when there is no file name.
pub fn display_name(path: &StandardizedPath) -> String {
    let local = path.to_local_path_lossy();
    local
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| local.to_string_lossy().into_owned())
}

/// The handles a bootstrapped `repo_explorer` session exposes to its driver
/// (the runtime in `main.rs`, or the assertions in the integration test).
pub struct Bootstrapped {
    pub window_id: WindowId,
    pub root: TuiViewHandle<RepoExplorerView>,
}

/// Registers the real model graph, indexes `std_path`, awaits the first scan on
/// the shared runtime, then installs the root view (observing the model so it
/// redraws on change). This is the single wiring path both the example binary
/// and the integration test exercise.
pub async fn bootstrap(
    app: &mut App,
    std_path: StandardizedPath,
    quit: Rc<Cell<bool>>,
) -> Bootstrapped {
    let repo_id = RepositoryIdentifier::local(std_path.clone());

    // Register the singletons the real model graph depends on (the same ones the
    // GUI app registers), then construct the real model via the shared core.
    app.add_singleton_model(DirectoryWatcher::new);
    app.add_singleton_model(|_| DetectedRepositories::default());
    let model = app.add_model(RepoMetadataModel::new);

    // Kick off indexing and await the first scan on the shared runtime so the
    // first frame already shows live metadata.
    let indexed = app.update(|ctx| {
        model.update(ctx, |model, ctx| {
            if let Err(error) = model.index_local_directory_path(&std_path, ctx) {
                eprintln!("failed to index {std_path}: {error}");
            }
            model.repository_indexed(&repo_id, ctx)
        })
    });
    indexed.await;

    let model_for_view = model.clone();
    let repo_id_for_view = repo_id.clone();
    let (window_id, root) = app.update(|ctx| {
        ctx.add_tui_window(window_options(), |view_ctx| {
            // Redraw whenever the model changes — the same observation primitive
            // GUI views use.
            view_ctx.observe(&model_for_view, |_view, _model, ctx| ctx.notify());
            RepoExplorerView {
                model: model_for_view.clone(),
                repo_id: repo_id_for_view.clone(),
                repo_root: std_path.clone(),
                selected: 0,
                quit: quit.clone(),
            }
        })
    });

    Bootstrapped { window_id, root }
}

fn window_options() -> AddWindowOptions {
    AddWindowOptions {
        window_style: WindowStyle::NotStealFocus,
        ..Default::default()
    }
}
