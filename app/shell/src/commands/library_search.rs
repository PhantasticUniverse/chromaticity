// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>

//! Scene-library search / filter Tauri IPC commands — Wave 4b.2 PR 2.
//!
//! Wraps the [`super::library_index::SceneIndex`] primitives shipped by
//! Wave 4b.2 PR 1 in `#[tauri::command]` handlers consumed by the
//! Library view's search box. Surfaces the `docs/38-scene-library-and-import-spec.md`
//! §2 search / filter axes per `docs/26-evidence-artifact-registry.md`
//! row 66 (`EV-LIBRARY-DISCOVERY`).
//!
//! ## Index lifecycle
//!
//! Each command call rebuilds the [`SceneIndex`] against the supplied
//! roots. Caching the index across calls via Tauri-managed state
//! (e.g. `AppState`-resident `Arc<RwLock<SceneIndex>>` + filesystem-
//! watch invalidation) is post-MVP per the PR 1 module prose; the
//! current cost is one directory walk per query, which is acceptable
//! for the in-MVP corpus size (the bundled `library/` corpus is three
//! `.chroma` packages plus whatever the user has authored).
//!
//! ## Root composition
//!
//! Both arguments mirror the existing
//! [`super::library::list_scene_packages`] /
//! [`super::library::list_bundled_scene_packages`] pair: `user_root` is
//! the platform-default writable scene root resolved by the frontend
//! via the `default_scene_root` command; `bundled_root` is the
//! `<AppBundle>/Resources/scenes/` path resolved via
//! `bundled_scene_root` and is `None` on platforms / build modes
//! where the bundled corpus is unavailable. Bundled root is walked
//! first so the dedup-by-id rule shadows any user-staged `.chroma`
//! that collides with a built-in demo's id.
//!
//! Unlike [`super::library::list_bundled_scene_packages`], the bundled
//! arm here is fed a single root path; the multi-subdirectory taxonomy
//! recursion (`library / welcome / examples`) is the responsibility of
//! the frontend caller, which composes one root entry per subdirectory
//! in practice. PR 2 keeps the IPC surface narrow to what the search
//! box actually needs: a flat list of roots fed straight into
//! [`SceneIndex::build`].

use chromaticity_ui_bridge::{SceneCard, SceneSource};
use std::path::PathBuf;

use super::library_index::SceneIndex;

/// Compose the `(PathBuf, SceneSource)` slice fed into
/// [`SceneIndex::build`] from the IPC arguments.
///
/// Bundled root is walked first so the dedup-by-id rule keeps the
/// built-in demo when a user-staged `.chroma` collides on id. A
/// `None` bundled root collapses to a user-only walk — the
/// platform-without-bundled-resources case.
fn compose_roots(user_root: String, bundled_root: Option<String>) -> Vec<(PathBuf, SceneSource)> {
    let mut roots = Vec::with_capacity(2);
    if let Some(bundled) = bundled_root {
        roots.push((PathBuf::from(bundled), SceneSource::Bundled));
    }
    roots.push((PathBuf::from(user_root), SceneSource::User));
    roots
}

/// Free-text scene-library search.
///
/// Builds a [`SceneIndex`] across `user_root` (always tagged
/// [`SceneSource::User`]) plus `bundled_root` (when present, tagged
/// [`SceneSource::Bundled`]) and returns every card whose `title`,
/// `credit`, `description`, any `tag`, or any `category` carries
/// `query` as a case-insensitive substring per
/// [`SceneIndex::search`]. An empty `query` returns every indexed
/// card (the degenerate "no filter" case).
///
/// Errors surface as the [`super::library_index::IndexError`]
/// `Display` form, which carries the offending root path for
/// diagnostics.
#[tauri::command]
pub fn search_scene_library(
    query: String,
    user_root: String,
    bundled_root: Option<String>,
) -> Result<Vec<SceneCard>, String> {
    let roots = compose_roots(user_root, bundled_root);
    let index = SceneIndex::build(&roots).map_err(|e| e.to_string())?;
    Ok(index.search(&query).into_iter().cloned().collect())
}

/// Filter the scene library by exact tag (case-insensitive).
///
/// Returns every card whose `tags` array contains `tag`
/// case-insensitively per [`SceneIndex::filter_by_tag`]. Substring
/// tag matching lives in [`search_scene_library`] — the dedicated
/// tag filter exists for the Library view's chip-style facet bar.
#[tauri::command]
pub fn filter_scene_library_by_tag(
    tag: String,
    user_root: String,
    bundled_root: Option<String>,
) -> Result<Vec<SceneCard>, String> {
    let roots = compose_roots(user_root, bundled_root);
    let index = SceneIndex::build(&roots).map_err(|e| e.to_string())?;
    Ok(index.filter_by_tag(&tag).into_iter().cloned().collect())
}

/// Filter the scene library by exact category (case-insensitive).
///
/// Mirrors [`filter_scene_library_by_tag`] against the `categories`
/// axis per [`SceneIndex::filter_by_category`]. The Library view's
/// BUILT-IN / IMPORT / USER chips read this command.
#[tauri::command]
pub fn filter_scene_library_by_category(
    category: String,
    user_root: String,
    bundled_root: Option<String>,
) -> Result<Vec<SceneCard>, String> {
    let roots = compose_roots(user_root, bundled_root);
    let index = SceneIndex::build(&roots).map_err(|e| e.to_string())?;
    Ok(index
        .filter_by_category(&category)
        .into_iter()
        .cloned()
        .collect())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-LIBRARY-DISCOVERY (T1) — pins the IPC command
    //! surface against the in-process [`SceneIndex`] primitives. The
    //! `#[tauri::command]` attribute is a procedural-macro shim around
    //! the underlying `pub fn`; the function body is callable directly
    //! from tests without spinning a Tauri runtime, which is the
    //! pattern the rest of `app/shell/src/commands/*` uses (cf.
    //! `library.rs` and `osc.rs`). The tier source is `docs/26` row 66
    //! `Generation Source` ("Library index suite") read against
    //! `docs/43` §15.5 — drift pins for the IPC wrappers qualify as
    //! T1 alongside the indexer pins shipped in PR 1.
    //!
    //! The corpus uses the repo-resident demo fixtures under
    //! `engine/scene/tests/fixtures/scenes/library/` — the same path
    //! `bundle.resources` maps into `<AppBundle>/Resources/scenes/library/`
    //! at link time per `bundle_layout_assertion.rs` — so the IPC
    //! commands exercise the production metadata-extraction contract
    //! without touching the user's real `$HOME`.
    //!
    //! The `commands_are_registered_in_invoke_handler` pin (in
    //! `lib.rs`-adjacent compile-time form) lives implicitly in the
    //! `tauri::generate_handler!` macro expansion: a stale handler
    //! registration would fail to compile because the macro
    //! type-checks every entry against the actual `#[tauri::command]`
    //! function signature.

    use super::{
        filter_scene_library_by_category, filter_scene_library_by_tag, search_scene_library,
    };
    use std::path::{Path, PathBuf};

    /// Repo-resident library fixture path (the same source-of-truth
    /// directory that `bundle.resources` maps into the AppBundle).
    fn library_fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../engine/scene/tests/fixtures/scenes/library")
    }

    /// Allocate a fresh empty scratch directory for the
    /// empty-user-root test arms.
    ///
    /// Uses a process-id-stamped path under [`std::env::temp_dir`]
    /// (mirroring the temp-file pattern at
    /// `search_command_handles_invalid_user_root_returns_io_error_string`
    /// below) so the empty-walk assertions do NOT couple to whichever
    /// repo-resident `engine/scene/tests/fixtures/scenes/<subdir>/`
    /// the corpus seeds at any given moment. The previous
    /// `welcome/`-as-empty-sentinel pattern broke when Wave 1d-β
    /// PR 1 seeded `welcome/welcome.chroma` under
    /// `EV-LIBRARY-CORPUS-MIN`. Directories are leaked under the
    /// process-id namespace; OS temp cleanup reclaims them. The
    /// helper returns the freshly-created directory path.
    fn fresh_empty_scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "chromaticity-library-search-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir must be created");
        dir
    }

    /// Convenience wrapper returning a fresh empty user-root path
    /// as an owned `String` for the IPC-surface tests.
    fn empty_user_root() -> String {
        fresh_empty_scratch_dir().to_string_lossy().into_owned()
    }

    #[test]
    fn search_command_returns_matching_cards() {
        // Hit demo-kaleido via a title substring; the bundled root
        // surfaces three demos so a single hit pins the search axis.
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        let hits = search_scene_library("kaleido".to_string(), user, Some(bundled))
            .expect("search must succeed against the fixture corpus");
        assert_eq!(hits.len(), 1, "exactly one title hit");
        assert_eq!(hits[0].id, "demo-kaleido");
    }

    #[test]
    fn search_command_returns_empty_vec_for_no_match() {
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        let hits = search_scene_library("__no_such_query__".to_string(), user, Some(bundled))
            .expect("no-match search must succeed");
        assert!(hits.is_empty(), "expected empty Vec, got {hits:?}");
    }

    #[test]
    fn search_command_handles_invalid_user_root_returns_io_error_string() {
        // A user root that is itself a file (not a directory) surfaces
        // as an `IoError` from the walker — the `read_dir` call inside
        // `list_scene_packages_impl` fails on non-directory inputs.
        // Stage a temp file to exercise that arm. (A nonexistent path
        // is handled gracefully as `Ok(empty)` by the walker, so we
        // need an existing-but-not-a-dir target to surface the error.)
        let temp_file = std::env::temp_dir().join(format!(
            "chromaticity-library-search-not-a-dir-{}.tmp",
            std::process::id()
        ));
        std::fs::write(&temp_file, b"not a directory").expect("temp file must be writable");

        let result = search_scene_library(
            "anything".to_string(),
            temp_file.to_string_lossy().into_owned(),
            None,
        );

        let _ = std::fs::remove_file(&temp_file);

        match result {
            Err(message) => {
                // The `IndexError::IoError` Display form starts with
                // "scene index I/O error under " — pin the prefix so
                // the surface is stable for frontend display.
                assert!(
                    message.contains("scene index I/O error under")
                        || message.contains("could not read"),
                    "expected IoError message, got: {message}",
                );
            }
            Ok(cards) => panic!("expected Err for non-directory input, got Ok({cards:?})"),
        }
    }

    #[test]
    fn search_command_with_empty_query_returns_every_card() {
        // Pin the empty-query no-filter degenerate case at the IPC
        // surface — matches `SceneIndex::search`'s contract.
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        let hits = search_scene_library("".to_string(), user, Some(bundled))
            .expect("empty-query search must succeed");
        assert!(
            hits.len() >= 3,
            "library corpus must surface at least three demos, got {}",
            hits.len(),
        );
    }

    #[test]
    fn search_command_without_bundled_root_walks_user_only() {
        // `bundled_root: None` collapses to a user-only walk —
        // exercises the `compose_roots` `None` arm. The user root is
        // a fresh empty scratch directory under `std::env::temp_dir`
        // so the result is the empty corpus regardless of what the
        // repo-resident `fixtures/scenes/` subdirs hold.
        let user = empty_user_root();
        let hits = search_scene_library("anything".to_string(), user, None)
            .expect("user-only search must succeed");
        assert!(hits.is_empty(), "fresh scratch dir → no cards");
    }

    #[test]
    fn filter_by_tag_command_returns_matching_cards() {
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        // demo-spectrum carries TAGS ["audio", "reactive"]; the
        // command must surface it via the tag-axis filter.
        let hits = filter_scene_library_by_tag("audio".to_string(), user, Some(bundled))
            .expect("tag filter must succeed");
        let ids: Vec<&str> = hits.iter().map(|c| c.id.as_str()).collect();
        assert!(
            ids.contains(&"demo-spectrum"),
            "demo-spectrum must surface for 'audio' tag, got {ids:?}",
        );
    }

    #[test]
    fn filter_by_tag_command_is_case_insensitive() {
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        let lower =
            filter_scene_library_by_tag("audio".to_string(), user.clone(), Some(bundled.clone()))
                .expect("lower tag filter must succeed");
        let upper = filter_scene_library_by_tag("AUDIO".to_string(), user, Some(bundled))
            .expect("upper tag filter must succeed");
        let ids_lower: Vec<&str> = lower.iter().map(|c| c.id.as_str()).collect();
        let ids_upper: Vec<&str> = upper.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids_lower, ids_upper,
            "tag filter must be case-insensitive at the IPC surface",
        );
    }

    #[test]
    fn filter_by_tag_command_unknown_tag_returns_empty() {
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        let hits = filter_scene_library_by_tag("__no_such_tag__".to_string(), user, Some(bundled))
            .expect("unknown-tag filter must succeed");
        assert!(hits.is_empty());
    }

    #[test]
    fn filter_by_category_command_returns_matching_cards() {
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        // Every demo carries the BUILT-IN category — the filter must
        // surface all three demos.
        let hits = filter_scene_library_by_category("BUILT-IN".to_string(), user, Some(bundled))
            .expect("category filter must succeed");
        assert!(
            hits.len() >= 3,
            "BUILT-IN must surface every demo, got {} hits",
            hits.len(),
        );
    }

    #[test]
    fn filter_by_category_command_narrows_to_geometric() {
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        // demo-kaleido carries GEOMETRIC; demo-spectrum does not.
        let hits = filter_scene_library_by_category("geometric".to_string(), user, Some(bundled))
            .expect("category filter must succeed");
        let ids: Vec<&str> = hits.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"demo-kaleido"));
        assert!(!ids.contains(&"demo-spectrum"));
    }

    #[test]
    fn filter_by_category_command_unknown_category_returns_empty() {
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        let hits = filter_scene_library_by_category(
            "__no_such_category__".to_string(),
            user,
            Some(bundled),
        )
        .expect("unknown-category filter must succeed");
        assert!(hits.is_empty());
    }

    #[test]
    fn compose_roots_orders_bundled_before_user() {
        // White-box pin: the dedup-by-id contract (PR 1) keeps the
        // first-seen card, so bundled MUST be walked before user so
        // a built-in demo wins over a user-staged collision. This
        // pin guards the helper against accidental reordering.
        let roots = super::compose_roots(
            "/tmp/user-root".to_string(),
            Some("/tmp/bundled-root".to_string()),
        );
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].1, chromaticity_ui_bridge::SceneSource::Bundled);
        assert_eq!(roots[1].1, chromaticity_ui_bridge::SceneSource::User);
    }

    #[test]
    fn compose_roots_user_only_when_bundled_none() {
        let roots = super::compose_roots("/tmp/user-root".to_string(), None);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].1, chromaticity_ui_bridge::SceneSource::User);
    }

    #[test]
    fn search_command_returns_owned_cards_clone_safe() {
        // The IPC layer returns Vec<SceneCard> (owned), not
        // Vec<&SceneCard> as the indexer surface does. Pin that the
        // command body clones correctly — a regression that returned
        // borrowed references would not compile against the
        // `#[tauri::command]` Serialize requirement, but a manual
        // assertion that the ids survive a drop of the source roots
        // catches accidental lifetime-shadowing in future refactors.
        let bundled = library_fixture_root().to_string_lossy().into_owned();
        let user = empty_user_root();
        let hits = search_scene_library("demo".to_string(), user, Some(bundled))
            .expect("search must succeed");
        let ids: Vec<String> = hits.into_iter().map(|c| c.id).collect();
        assert!(!ids.is_empty(), "demo query must surface at least one card");
    }
}
