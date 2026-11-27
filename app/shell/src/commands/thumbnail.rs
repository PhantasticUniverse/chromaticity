// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Scene library thumbnail pipeline — Wave 4b.3 Lane A.
//!
//! Implements the cache-lookup → render-on-miss → placeholder-fallback
//! chain for `EV-LIBRARY-THUMBNAIL` per
//! `docs/26-evidence-artifact-registry.md` row 67 and
//! `docs/38-scene-library-and-import-spec.md` §3 (library thumbnails).
//!
//! ## Pipeline
//!
//! ```text
//! get_or_render(key, scene)
//!   ├─ cache hit                  →  ThumbnailEntry { Cached     }
//!   ├─ cache miss → renderer ok   →  ThumbnailEntry { Rendered   }   then write-through
//!   └─ cache miss → renderer err  →  ThumbnailEntry { Placeholder }  (PLACEHOLDER_BYTES)
//! ```
//!
//! ## Determinism contract
//!
//! [`ThumbnailKey`] is derived from `(scene_id, format_version)`. The
//! cache file path is `<cache_root>/<sha256(serialised key)>.thumb`,
//! and the production-deferred [`DeterministicStubRenderer`] hashes the
//! same serialisation so a given `(scene_id, format_version)` regenerates
//! to byte-identical thumbnail bytes across hosts and across runs.
//!
//! ## Scope (lane A — engine-side pipeline only)
//!
//! - Tauri-side `#[tauri::command]` IPC wrappers are intentionally NOT
//!   part of this PR; the Library view will gain a fetch surface in a
//!   later closing-cluster PR.
//! - The renderer is abstracted behind [`ThumbnailRenderer`] so a real
//!   GPU-based implementation can swap in once the rendering pipeline
//!   matures (`engine/render/`). The shipped
//!   [`DeterministicStubRenderer`] returns a 32-byte SHA-256 digest of
//!   the key, which is sufficient to exercise byte-equality across the
//!   determinism path without coupling to a GPU surface.
//! - Eviction is single-key only ([`ThumbnailCache::evict`]); bulk /
//!   LRU eviction lands when the rendered-image-size cost surfaces.
//!
//! ## File layout
//!
//! Each thumbnail is one file: `<cache_root>/<hash>.thumb`. Writes are
//! atomic via the `<hash>.thumb.tmp` → rename pattern so a crash
//! mid-write cannot leave a partial cache entry that a subsequent
//! lookup would happily return.
//!
//! ## Error surface
//!
//! [`ThumbnailError`] is `#[non_exhaustive]` so the variant set can
//! grow (e.g. `RendererTimeout` when a real GPU renderer lands)
//! without breaking downstream matches.

use chromaticity_ui_bridge::SceneCard;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Static placeholder bytes used as the final fallback when the
/// renderer surfaces an error. Intentionally a small fixed sequence
/// (the ASCII bytes of `chromaticity-thumb-placeholder-v1`) so cache
/// callers can `==`-check the placeholder arm in tests without
/// depending on PNG-decoder availability. A future PR may swap this
/// for a real placeholder PNG once the Library view's render surface
/// requires it.
pub static PLACEHOLDER_BYTES: &[u8] = b"chromaticity-thumb-placeholder-v1";

/// Provenance of a [`ThumbnailEntry`]: how the bytes were obtained.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ThumbnailSource {
    /// Bytes were read from the on-disk cache (fast path).
    Cached,
    /// Bytes were freshly produced by the renderer and then written
    /// through to the cache.
    Rendered,
    /// The renderer surfaced an error; bytes are
    /// [`PLACEHOLDER_BYTES`].
    Placeholder,
}

/// Cache key for a single thumbnail.
///
/// Derived from `(scene_id, format_version)` so the same scene at the
/// same format version produces a stable on-disk hash. `format_version`
/// is supplied by the caller and typically composes the scene's
/// `SCHEMA_VERSION` + `format_tag` so a schema bump invalidates every
/// cached thumbnail naturally without an explicit purge.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbnailKey {
    /// Scene id (matches [`SceneCard::id`]).
    pub scene_id: String,
    /// Opaque version string the caller bumps when the scene's visual
    /// identity changes (e.g. on schema bump).
    pub format_version: String,
}

impl ThumbnailKey {
    /// Stable string serialisation used both as the cache-file basename
    /// (after sha256 hashing) and as the input to the deterministic
    /// stub renderer. The colon separator is safe because scene ids
    /// per `docs/38` §2.2 are kebab-case ASCII and format versions are
    /// numeric / alphanumeric.
    pub fn to_stable_string(&self) -> String {
        format!("{}:{}", self.scene_id, self.format_version)
    }

    /// Filename-safe hash of [`Self::to_stable_string`]. 64-char
    /// lowercase hex SHA-256 digest with the `.thumb` extension
    /// appended at the cache layer.
    fn cache_basename(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.to_stable_string().as_bytes());
        let digest = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in digest {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex
    }
}

/// One thumbnail lookup result: the bytes plus a provenance tag.
#[derive(Debug, Clone)]
pub struct ThumbnailEntry {
    /// Raw thumbnail bytes (renderer output, cached read-back, or
    /// `PLACEHOLDER_BYTES`).
    pub bytes: Vec<u8>,
    /// Where the bytes came from in the fallback chain.
    pub source: ThumbnailSource,
}

/// Errors the thumbnail pipeline surfaces.
///
/// `#[non_exhaustive]` so renderer-timeout / quota-exceeded variants
/// can land without breaking downstream matches.
#[non_exhaustive]
#[derive(Debug)]
pub enum ThumbnailError {
    /// I/O error reading or writing the cache file.
    CacheIoError(io::Error),
    /// Renderer reported a non-IO failure; the cache returns the
    /// placeholder fallback rather than propagating this variant.
    /// Surfaced separately so direct renderer callers (tests, future
    /// pre-warm pipelines) can observe the failure mode without going
    /// through the cache's silent-fallback contract.
    RenderError(String),
    /// Cache key violated an invariant (e.g. empty scene id).
    KeyError(String),
}

impl fmt::Display for ThumbnailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThumbnailError::CacheIoError(err) => write!(f, "thumbnail cache I/O error: {err}"),
            ThumbnailError::RenderError(msg) => write!(f, "thumbnail render error: {msg}"),
            ThumbnailError::KeyError(msg) => write!(f, "thumbnail key error: {msg}"),
        }
    }
}

impl std::error::Error for ThumbnailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ThumbnailError::CacheIoError(err) => Some(err),
            _ => None,
        }
    }
}

/// Trait abstracting the render half of the pipeline so the cache can
/// be exercised against a deterministic stub today and a real
/// GPU-backed implementation later.
///
/// Implementations must be `Send + Sync` because the cache is intended
/// to live behind a Tauri `State` once the IPC wrapper lands.
pub trait ThumbnailRenderer: Send + Sync {
    /// Produce thumbnail bytes for `key` using `scene` as authoring-
    /// input context. The byte format is implementation-defined; the
    /// pipeline treats the result as opaque.
    fn render(&self, key: &ThumbnailKey, scene: &SceneCard) -> Result<Vec<u8>, ThumbnailError>;
}

/// Deterministic-bytes stand-in for the future GPU-backed renderer.
///
/// Hashes the [`ThumbnailKey::to_stable_string`] form via SHA-256 and
/// returns the 32-byte digest. The output is byte-identical for a
/// given key across processes and hosts, which is the property the
/// `EV-LIBRARY-THUMBNAIL` deterministic-regeneration contract pins.
///
/// This stub will be replaced by a real GPU-rendered preview when the
/// rendering pipeline matures (`engine/render/` is intentionally NOT
/// imported here so this PR's diff stays small).
pub struct DeterministicStubRenderer;

impl ThumbnailRenderer for DeterministicStubRenderer {
    fn render(&self, key: &ThumbnailKey, _scene: &SceneCard) -> Result<Vec<u8>, ThumbnailError> {
        if key.scene_id.is_empty() {
            return Err(ThumbnailError::KeyError(
                "scene_id must be non-empty".into(),
            ));
        }
        let mut hasher = Sha256::new();
        hasher.update(key.to_stable_string().as_bytes());
        Ok(hasher.finalize().to_vec())
    }
}

/// On-disk thumbnail cache with the documented fallback chain.
///
/// Owns the renderer (boxed) so callers can swap implementations
/// without re-plumbing the cache's public surface.
pub struct ThumbnailCache {
    cache_root: PathBuf,
    renderer: Box<dyn ThumbnailRenderer>,
}

impl ThumbnailCache {
    /// Construct a cache rooted at `cache_root`, dispatching to
    /// `renderer` on cache miss. The directory is created lazily on
    /// the first write — passing a path that does not yet exist is
    /// supported and is the expected fresh-install case.
    pub fn new(cache_root: PathBuf, renderer: Box<dyn ThumbnailRenderer>) -> Self {
        Self {
            cache_root,
            renderer,
        }
    }

    /// Compute the on-disk path for `key`'s thumbnail file.
    fn cache_path(&self, key: &ThumbnailKey) -> PathBuf {
        let mut name = key.cache_basename();
        name.push_str(".thumb");
        self.cache_root.join(name)
    }

    /// Lookup-then-render-on-miss-then-cache. Falls back to
    /// [`PLACEHOLDER_BYTES`] when the renderer surfaces an error so the
    /// Library view can always surface *something* per the
    /// `docs/38` §3 fallback contract.
    pub fn get_or_render(
        &self,
        key: &ThumbnailKey,
        scene: &SceneCard,
    ) -> Result<ThumbnailEntry, ThumbnailError> {
        if key.scene_id.is_empty() {
            return Err(ThumbnailError::KeyError(
                "scene_id must be non-empty".into(),
            ));
        }
        let path = self.cache_path(key);
        if path.exists() {
            match fs::read(&path) {
                Ok(bytes) => {
                    return Ok(ThumbnailEntry {
                        bytes,
                        source: ThumbnailSource::Cached,
                    });
                }
                Err(err) => return Err(ThumbnailError::CacheIoError(err)),
            }
        }

        match self.renderer.render(key, scene) {
            Ok(bytes) => {
                self.write_atomic(&path, &bytes)?;
                Ok(ThumbnailEntry {
                    bytes,
                    source: ThumbnailSource::Rendered,
                })
            }
            Err(_) => Ok(ThumbnailEntry {
                bytes: PLACEHOLDER_BYTES.to_vec(),
                source: ThumbnailSource::Placeholder,
            }),
        }
    }

    /// Remove the cache entry for `key` if present. Missing entries
    /// are not an error — the post-condition is "no file at the cache
    /// path", which a missing entry already satisfies.
    pub fn evict(&self, key: &ThumbnailKey) -> Result<(), ThumbnailError> {
        let path = self.cache_path(key);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(ThumbnailError::CacheIoError(err)),
        }
    }

    /// Atomic write: emit `<path>.tmp` then rename onto `<path>`. A
    /// crash between create and rename leaves the `.tmp` orphan but
    /// never a partial `.thumb`, so a subsequent lookup either finds
    /// the previous good entry or re-renders.
    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), ThumbnailError> {
        if !self.cache_root.exists() {
            fs::create_dir_all(&self.cache_root).map_err(ThumbnailError::CacheIoError)?;
        }
        let mut tmp = path.to_path_buf();
        let mut tmp_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_owned(),
            None => {
                return Err(ThumbnailError::CacheIoError(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "cache path has no file name",
                )));
            }
        };
        tmp_name.push_str(".tmp");
        tmp.set_file_name(tmp_name);
        fs::write(&tmp, bytes).map_err(ThumbnailError::CacheIoError)?;
        fs::rename(&tmp, path).map_err(ThumbnailError::CacheIoError)?;
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-LIBRARY-THUMBNAIL (T1) — drift pins for the
    //! cache-lookup → render-on-miss → placeholder fallback chain plus
    //! the deterministic-regen contract per
    //! `docs/26-evidence-artifact-registry.md` row 67. Tier source:
    //! `docs/26` row 67 `Generation Source` ("Thumbnail pipeline +
    //! deterministic-regen corpus") read against `docs/43` §15.5 —
    //! in-crate drift pins for the pipeline qualify as T1.
    //!
    //! The fixture-backed cross-crate corpus that exercises the
    //! pipeline against the real bundled scenes lives at
    //! `app/shell/tests/thumbnail_pipeline_corpus.rs` (T2).
    use super::*;
    use chromaticity_ui_bridge::{SceneCard, SceneSource};

    fn synthetic_scene(id: &str) -> SceneCard {
        SceneCard {
            id: id.to_string(),
            title: format!("Synthetic {id}"),
            credit: "test".into(),
            description: "fixture".into(),
            tags: vec!["fixture".into()],
            categories: vec!["TEST".into()],
            path: format!("/dev/null/{id}"),
            format_tag: "native-chroma-scene".into(),
            source: SceneSource::Bundled,
        }
    }

    /// Per-test isolated cache root; uses the process id + the test
    /// name so parallel `cargo test` runs do not collide.
    fn fresh_cache_root(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "chromaticity-thumb-cache-{}-{}",
            std::process::id(),
            tag
        ));
        // Best-effort cleanup of any leftover from a previous failed
        // run; ignore errors — `get_or_render` will recreate the dir.
        let _ = fs::remove_dir_all(&p);
        p
    }

    fn stub_cache(tag: &str) -> ThumbnailCache {
        ThumbnailCache::new(fresh_cache_root(tag), Box::new(DeterministicStubRenderer))
    }

    #[test]
    fn deterministic_renderer_produces_same_bytes_for_same_key() {
        let renderer = DeterministicStubRenderer;
        let key = ThumbnailKey {
            scene_id: "demo-kaleido".into(),
            format_version: "1".into(),
        };
        let scene = synthetic_scene("demo-kaleido");
        let a = renderer.render(&key, &scene).unwrap();
        let b = renderer.render(&key, &scene).unwrap();
        let c = renderer.render(&key, &scene).unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.len(), 32, "SHA-256 digest is 32 bytes");
    }

    #[test]
    fn deterministic_renderer_produces_different_bytes_for_different_keys() {
        let renderer = DeterministicStubRenderer;
        let scene = synthetic_scene("demo");
        let a = renderer
            .render(
                &ThumbnailKey {
                    scene_id: "a".into(),
                    format_version: "1".into(),
                },
                &scene,
            )
            .unwrap();
        let b = renderer
            .render(
                &ThumbnailKey {
                    scene_id: "b".into(),
                    format_version: "1".into(),
                },
                &scene,
            )
            .unwrap();
        let c = renderer
            .render(
                &ThumbnailKey {
                    scene_id: "a".into(),
                    format_version: "2".into(),
                },
                &scene,
            )
            .unwrap();
        assert_ne!(a, b, "different scene_id → different bytes");
        assert_ne!(a, c, "different format_version → different bytes");
        assert_ne!(b, c);
    }

    #[test]
    fn deterministic_renderer_rejects_empty_scene_id() {
        let renderer = DeterministicStubRenderer;
        let scene = synthetic_scene("demo");
        let err = renderer
            .render(
                &ThumbnailKey {
                    scene_id: "".into(),
                    format_version: "1".into(),
                },
                &scene,
            )
            .unwrap_err();
        assert!(matches!(err, ThumbnailError::KeyError(_)));
    }

    #[test]
    fn cache_miss_falls_back_to_render_then_caches() {
        let cache = stub_cache("miss-then-cache");
        let key = ThumbnailKey {
            scene_id: "demo-kaleido".into(),
            format_version: "1".into(),
        };
        let scene = synthetic_scene("demo-kaleido");
        let first = cache.get_or_render(&key, &scene).unwrap();
        assert_eq!(first.source, ThumbnailSource::Rendered);
        // Second call must hit the cache (the write-through populated
        // the on-disk entry).
        let second = cache.get_or_render(&key, &scene).unwrap();
        assert_eq!(second.source, ThumbnailSource::Cached);
        assert_eq!(first.bytes, second.bytes);
    }

    #[test]
    fn cache_lookup_returns_cached_entry_when_present() {
        let cache = stub_cache("cached-when-present");
        let key = ThumbnailKey {
            scene_id: "demo-text".into(),
            format_version: "1".into(),
        };
        let scene = synthetic_scene("demo-text");
        // Prime the cache.
        let _ = cache.get_or_render(&key, &scene).unwrap();
        // Subsequent reads must surface as Cached without re-running
        // the renderer (we cannot count renderer invocations through
        // the trait object directly, but the source tag is the
        // contract surface).
        for _ in 0..3 {
            let entry = cache.get_or_render(&key, &scene).unwrap();
            assert_eq!(entry.source, ThumbnailSource::Cached);
        }
    }

    /// Renderer that always errors — the cache must surface
    /// [`ThumbnailSource::Placeholder`] with [`PLACEHOLDER_BYTES`].
    struct ErrRenderer;
    impl ThumbnailRenderer for ErrRenderer {
        fn render(
            &self,
            _key: &ThumbnailKey,
            _scene: &SceneCard,
        ) -> Result<Vec<u8>, ThumbnailError> {
            Err(ThumbnailError::RenderError("synthetic failure".into()))
        }
    }

    #[test]
    fn cache_get_or_render_uses_placeholder_when_renderer_errors() {
        let cache = ThumbnailCache::new(
            fresh_cache_root("err-renderer-placeholder"),
            Box::new(ErrRenderer),
        );
        let key = ThumbnailKey {
            scene_id: "demo".into(),
            format_version: "1".into(),
        };
        let scene = synthetic_scene("demo");
        let entry = cache.get_or_render(&key, &scene).unwrap();
        assert_eq!(entry.source, ThumbnailSource::Placeholder);
        assert_eq!(entry.bytes, PLACEHOLDER_BYTES);
        // Placeholder must NOT be written through to the cache; a
        // second call still hits the fallback.
        let entry2 = cache.get_or_render(&key, &scene).unwrap();
        assert_eq!(entry2.source, ThumbnailSource::Placeholder);
    }

    #[test]
    fn cache_evict_removes_entry() {
        let cache = stub_cache("evict-removes");
        let key = ThumbnailKey {
            scene_id: "demo".into(),
            format_version: "1".into(),
        };
        let scene = synthetic_scene("demo");
        let _ = cache.get_or_render(&key, &scene).unwrap();
        assert!(cache.cache_path(&key).exists());
        cache.evict(&key).unwrap();
        assert!(!cache.cache_path(&key).exists());
        // Evicting a missing key is a no-op (not an error).
        cache.evict(&key).unwrap();
        // After eviction the next lookup must re-render (Rendered, not
        // Cached) — pins the "fresh after evict" contract.
        let entry = cache.get_or_render(&key, &scene).unwrap();
        assert_eq!(entry.source, ThumbnailSource::Rendered);
    }

    #[test]
    fn thumbnail_key_serializes_to_stable_string() {
        let key = ThumbnailKey {
            scene_id: "demo-kaleido".into(),
            format_version: "1".into(),
        };
        // Stable, documented form: `<scene_id>:<format_version>`.
        assert_eq!(key.to_stable_string(), "demo-kaleido:1");
        // Round-trip via the basename hash: identical keys hash to
        // identical basenames, distinct keys to distinct basenames.
        let same = ThumbnailKey {
            scene_id: "demo-kaleido".into(),
            format_version: "1".into(),
        };
        assert_eq!(key.cache_basename(), same.cache_basename());
        let other = ThumbnailKey {
            scene_id: "demo-kaleido".into(),
            format_version: "2".into(),
        };
        assert_ne!(key.cache_basename(), other.cache_basename());
        // Basename is exactly 64 hex chars (SHA-256).
        assert_eq!(key.cache_basename().len(), 64);
        assert!(key.cache_basename().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn cache_handles_missing_cache_root_gracefully() {
        // Caller passes a path that does not exist; the cache must
        // create it on first write rather than erroring.
        let mut root = std::env::temp_dir();
        root.push(format!(
            "chromaticity-thumb-cache-missing-{}-nested/sub/dir",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        assert!(!root.exists());
        let cache = ThumbnailCache::new(root.clone(), Box::new(DeterministicStubRenderer));
        let key = ThumbnailKey {
            scene_id: "demo".into(),
            format_version: "1".into(),
        };
        let scene = synthetic_scene("demo");
        let entry = cache.get_or_render(&key, &scene).unwrap();
        assert_eq!(entry.source, ThumbnailSource::Rendered);
        assert!(root.exists(), "cache root must be created on first write");
    }

    #[test]
    fn cache_uses_atomic_write_to_avoid_partial_files() {
        // Pin the .tmp-then-rename pattern by checking that no .tmp
        // file survives a successful write.
        let cache = stub_cache("atomic-write");
        let key = ThumbnailKey {
            scene_id: "demo".into(),
            format_version: "1".into(),
        };
        let scene = synthetic_scene("demo");
        let _ = cache.get_or_render(&key, &scene).unwrap();
        let final_path = cache.cache_path(&key);
        assert!(final_path.exists());
        let mut tmp = final_path.clone();
        let mut name = final_path.file_name().unwrap().to_str().unwrap().to_owned();
        name.push_str(".tmp");
        tmp.set_file_name(name);
        assert!(
            !tmp.exists(),
            ".tmp orphan must not survive a successful write: {tmp:?}"
        );
    }

    #[test]
    fn thumbnail_error_implements_send_sync_static() {
        // IPC-bridging compatibility: ThumbnailError must be threadable
        // when the future Tauri command surface lands.
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}
        assert_send_sync_static::<ThumbnailError>();
    }

    #[test]
    fn thumbnail_source_variants_are_distinct() {
        // Cheap drift pin against an accidental refactor that
        // collapses two sources into one.
        assert_ne!(ThumbnailSource::Cached, ThumbnailSource::Rendered);
        assert_ne!(ThumbnailSource::Rendered, ThumbnailSource::Placeholder);
        assert_ne!(ThumbnailSource::Cached, ThumbnailSource::Placeholder);
    }
}
