//! Headless integration tests for the `repo_explorer` example (Task 4.3).
//!
//! These drive the **same** view + model wiring the interactive example uses by
//! `#[path]`-including its `view` module, so there is no forked logic. Each test
//! builds a TUI-backend `App`, adds the real `repo_metadata` model + the
//! example's root view, renders one pass into an in-memory `TuiBuffer`, and
//! asserts the rendered cells reflect real model state — and that a typed action
//! dispatched through the shared core changes the rendered state.

#[path = "../examples/repo_explorer/view.rs"]
mod view;

use std::cell::Cell;
use std::rc::Rc;

use warp_util::standardized_path::StandardizedPath;
use warpui_core::App;
use warpui_tui::{TuiPresenter, TuiRect};

use crate::view::{bootstrap, NavAction};

#[test]
fn buffer_reflects_real_model_state() {
    App::test((), |mut app| async move {
        let dir = std::env::current_dir().expect("cwd");
        let std_path = StandardizedPath::try_from_local(&dir).expect("standardized path");
        let quit = Rc::new(Cell::new(false));

        let session = bootstrap(&mut app, std_path, quit).await;

        // Render into a buffer tall enough to hold the indexed entries.
        let mut presenter = TuiPresenter::new();
        let area = TuiRect::new(0, 0, 100, 120);
        let frame = app.read(|ctx| presenter.present(ctx, &session.root, area));
        let text = frame.buffer.to_lines().join("\n");

        assert!(
            text.contains("repo_metadata"),
            "buffer should render the header sourced from the model:\n{text}"
        );
        assert!(
            text.contains("status: indexed"),
            "buffer should reflect the model's indexed state:\n{text}"
        );

        // The rendered list must be sourced from the model: take the model's own
        // first entry and assert it appears in the painted buffer. This is robust
        // to the model's (unstable) traversal order.
        let (first_name, entry_count) = app.read(|ctx| {
            session.root.read(ctx, |view, ctx| {
                let entries = view.entries(ctx);
                (entries.first().map(|(name, _)| name.clone()), entries.len())
            })
        });
        assert!(entry_count > 0, "the indexed repo should expose entries");
        let first_name = first_name.expect("there is at least one entry");
        assert!(
            text.contains(&first_name),
            "buffer should render the model's first entry {first_name:?}:\n{text}"
        );
    });
}

#[test]
fn typed_nav_action_changes_rendered_buffer() {
    App::test((), |mut app| async move {
        let dir = std::env::current_dir().expect("cwd");
        let std_path = StandardizedPath::try_from_local(&dir).expect("standardized path");
        let quit = Rc::new(Cell::new(false));

        let session = bootstrap(&mut app, std_path, quit).await;

        let mut presenter = TuiPresenter::new();
        let area = TuiRect::new(0, 0, 80, 40);

        // First frame: selection starts at entry 0.
        let before = app.read(|ctx| presenter.present(ctx, &session.root, area));
        let before_lines = before.buffer.to_lines();
        let selected_before = app.read(|ctx| session.root.read(ctx, |view, _| view.selected()));
        assert_eq!(
            selected_before, 0,
            "selection should start at the first entry"
        );

        // Dispatch the example's typed action through the shared core, exactly as
        // the runtime does when a navigation key is pressed.
        app.dispatch_typed_action(
            session.window_id,
            &[session.root.id()],
            &NavAction::SelectNext,
        );

        let selected_after = app.read(|ctx| session.root.read(ctx, |view, _| view.selected()));
        assert_eq!(
            selected_after, 1,
            "SelectNext dispatched through the shared core should advance the selection"
        );

        // Second frame: the rendered buffer must change (the selection marker moved).
        let after = app.read(|ctx| presenter.present(ctx, &session.root, area));
        let after_lines = after.buffer.to_lines();
        assert_ne!(
            before_lines, after_lines,
            "the typed action should change the rendered buffer"
        );

        // The selection marker '›' should sit on a different row after navigation.
        let marker_row = |lines: &[String]| lines.iter().position(|line| line.contains('›'));
        assert_ne!(
            marker_row(&before_lines),
            marker_row(&after_lines),
            "the selection marker should move to a different row"
        );
    });
}
