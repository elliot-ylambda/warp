//! Task 4.1 compile probe: forces the real `repo_metadata` model to compile and
//! construct under the **TUI** backend.
//!
//! Because `warpui_tui` enables `warpui_core/tui`, depending on `repo_metadata`
//! here makes Cargo feature-unification compile `repo_metadata` against
//! `AppContextImpl<TuiBackend>`. If any `repo_metadata` entry point reached for
//! the example were GUI-only, this probe would fail to build. It constructs the
//! singleton model through the shared `ModelContext`, indexes the launch
//! directory, awaits the async scan on the shared runtime, and prints what the
//! model reports — exercising the exact API the example drives.

use repo_metadata::local_model::GetContentsArgs;
use repo_metadata::repositories::DetectedRepositories;
use repo_metadata::watcher::DirectoryWatcher;
use repo_metadata::{RepoMetadataModel, RepositoryIdentifier};
use warp_util::standardized_path::StandardizedPath;
use warpui_core::App;

fn main() {
    App::test((), |mut app| async move {
        let dir = std::env::current_dir().expect("cwd");
        let std_path = StandardizedPath::try_from_local(&dir).expect("standardized path");
        let id = RepositoryIdentifier::local(std_path.clone());

        // The real model graph depends on these singletons (registered the same
        // way the GUI app registers them). They must exist before the model
        // subscribes to repository detection.
        app.add_singleton_model(DirectoryWatcher::new);
        app.add_singleton_model(|_| DetectedRepositories::default());

        // Construct the real model via the shared `ModelContext`/`add_model`.
        let model = app.add_model(RepoMetadataModel::new);

        // Index the launch directory and await the async scan on the shared
        // runtime (the same primitive a view's `spawn` would use).
        let indexed = app.update(|ctx| {
            model.update(ctx, |model, ctx| {
                model
                    .index_local_directory_path(&std_path, ctx)
                    .expect("index launch dir");
                model.repository_indexed(&id, ctx)
            })
        });
        indexed.await;

        app.read(|ctx| {
            let model = model.as_ref(ctx);
            let has = model.has_repository(&id, ctx);
            let count = model
                .get_repo_contents(&id, GetContentsArgs::default(), ctx)
                .map(|contents| contents.contents.len())
                .unwrap_or(0);
            println!("repo_metadata under TUI backend: indexed={has} entries={count}");
        });
    });
}
