//! `repo_explorer` — the saga's payoff example.
//!
//! A runnable terminal UI that reuses the **real** `repo_metadata` model through
//! the shared `warpui_core` machinery and renders it with the `warpui_tui` TUI
//! backend (elements + presenter + runtime).
//!
//! What it demonstrates (the same primitives a GUI view uses):
//! - Model reuse: it constructs the actual `RepoMetadataModel` via the shared
//!   `ModelContext`/`add_model`, indexes the directory it is launched in, and
//!   reads live metadata back. No model logic is re-implemented here.
//! - The shared async runtime: indexing runs on the background executor and is
//!   awaited on the foreground executor — the same `spawn` plumbing GUI views use.
//! - Model observation: the root view `observe`s the model so model changes mark
//!   it dirty and trigger a redraw.
//! - Typed-action dispatch: arrow/`j`/`k` keys dispatch a `NavAction` through the
//!   shared core to the view's `handle_action`, moving the selection.
//!
//! Keys: `j`/`Down` select next · `k`/`Up` select previous · `q`/`Esc` quit.
//!
//! Run with: `cargo run -p warpui_tui --example repo_explorer`

mod view;

use std::cell::Cell;
use std::io;
use std::rc::Rc;

use warp_util::standardized_path::StandardizedPath;
use warpui_core::App;
use warpui_tui::{CrosstermTerminal, TuiRuntime};

use crate::view::bootstrap;

async fn run(mut app: App) -> io::Result<()> {
    let dir = std::env::current_dir()?;
    let std_path = StandardizedPath::try_from_local(&dir)
        .map_err(|error| io::Error::other(format!("invalid launch directory: {error}")))?;

    let quit = Rc::new(Cell::new(false));
    let session = bootstrap(&mut app, std_path, quit.clone()).await;

    let terminal = CrosstermTerminal::enter()?;
    let mut runtime = TuiRuntime::with_terminal(&app, session.window_id, session.root, terminal);
    runtime.run_until(&mut app, |_| quit.get())?;
    Ok(())
}

fn main() -> io::Result<()> {
    App::test((), |app| async move { run(app).await })
}
