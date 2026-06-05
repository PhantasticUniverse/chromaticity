// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Main-thread output-window + surface bootstrap for ADR-011 Phase-2
//! Slice R-1a.
//!
//! [`build_output_window_resources`] runs ON THE MAIN THREAD (called
//! from the Tauri `setup` callback) and performs every operation that
//! is main-thread-only on macOS / Metal:
//!
//! 1. builds a plain [`tauri::window::Window`] (`"gpu-output"`) with NO
//!    webview — a Tauri `WebviewWindow` hosts a `WKWebView` that paints
//!    OPAQUE white over the wgpu Metal layer (an `about:blank` webview
//!    hides every presented frame), so the output window is webview-free,
//! 2. wraps it in an `Arc` and creates a `wgpu::Surface<'static>` from
//!    it (the `Arc<Window>` satisfies wgpu's `WindowHandle`
//!    super-trait via the `raw-window-handle` 0.6 `Arc` blanket impl,
//!    and is `Send + Sync + 'static` so the surface is `'static`),
//! 3. acquires a surface-compatible `wgpu::Adapter` from the SAME
//!    shell-side instance (for surface capability / default-config
//!    queries — also main-thread on macOS),
//! 4. MOVES that same instance into a surface-compatible
//!    [`chromaticity_render::MultipassRenderer`] via
//!    [`chromaticity_render::MultipassRenderer::new_with_surface_hint`]
//!    so the renderer's device shares the surface's instance (a device
//!    from a *different* instance panics "Surface does not exist" on
//!    configure/present); the renderer requests its own
//!    surface-compatible adapter + device from that instance (also
//!    main-thread on macOS), and
//! 5. builds the [`crate::render::blit::BlitPass`] against the chosen
//!    swapchain format.
//!
//! The returned [`RenderThreadResources`] is `Send`, so the caller
//! immediately hands it to [`crate::render::thread::spawn`], which
//! MOVES it onto a dedicated render thread. No surface / window / device
//! creation EVER happens off the main thread.
//!
//! ## One instance — load-bearing
//!
//! The shell creates exactly ONE `wgpu::Instance`; the surface, the
//! shell-side caps adapter, and the renderer's adapter + device all come
//! from it. This is mandatory, not an optimisation: wgpu tracks
//! resources in per-instance storage, so a renderer device built from a
//! *different* instance panics `Surface ... does not exist` the moment
//! it configures or acquires the surface on the render thread.
//!
//! `wgpu::Surface::get_capabilities` / `get_default_config` need a
//! `&wgpu::Adapter`, which the [`chromaticity_render::MultipassRenderer`]
//! keeps private (the R-0 surface API is intentionally frozen). Rather
//! than widen the engine API for R-1a, the shell requests its own
//! surface-compatible adapter (from the shared instance) purely for the
//! capability / default-config reads, then MOVES the instance into the
//! renderer (which requests its own surface-compatible adapter + device
//! from it for the actual configure + submit). On a single-GPU host
//! (every shipping macOS target) both `compatible_surface`-constrained
//! requests resolve to the identical Metal adapter, so the format /
//! present-mode the caps query reports are exactly what the renderer's
//! device can present. If R-1b needs the renderer's exact adapter, the
//! engine API widens then.
//!
//! ## No `unsafe`
//!
//! `wgpu::Instance::create_surface` is the SAFE surface entry point;
//! the `unsafe` raw-handle extraction lives INSIDE wgpu + tauri, not in
//! this crate. The workspace `unsafe_code = "forbid"` lint is inherited
//! and this module adds none.

use std::sync::Arc;

use chromaticity_render::MultipassRenderer;
use tauri::window::WindowBuilder;
use tauri::{Manager, Runtime};

use crate::render::blit::{BlitPass, preferred_surface_format};
use crate::render::thread::RenderThreadResources;

/// Stable window label for the R-1a output window.
const OUTPUT_WINDOW_LABEL: &str = "gpu-output";

/// Logical default size for the output window (physical size is read
/// back for the swapchain configuration).
const DEFAULT_LOGICAL_WIDTH: f64 = 1280.0;
const DEFAULT_LOGICAL_HEIGHT: f64 = 720.0;

/// Errors building the output-window render resources on the main
/// thread.
#[derive(Debug)]
pub enum OutputWindowError {
    /// The Tauri window builder failed (label clash, platform refusal).
    WindowBuild(tauri::Error),
    /// Reading the window's physical size failed.
    WindowSize(tauri::Error),
    /// `wgpu::Instance::create_surface` failed (unsupported handle).
    SurfaceCreate(wgpu::CreateSurfaceError),
    /// No shell-side adapter was compatible with the surface.
    AdapterUnavailable,
    /// The surface-compatible renderer could not be built (no
    /// compatible adapter / device-request rejected).
    Renderer(chromaticity_render::RenderError),
    /// The surface advertised no usable default configuration for the
    /// chosen adapter (no formats / present modes).
    NoSurfaceConfig,
}

impl core::fmt::Display for OutputWindowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WindowBuild(e) => write!(f, "output window build failed: {e}"),
            Self::WindowSize(e) => write!(f, "output window size query failed: {e}"),
            Self::SurfaceCreate(e) => write!(f, "wgpu surface creation failed: {e}"),
            Self::AdapterUnavailable => {
                write!(f, "no wgpu adapter is compatible with the output surface")
            }
            Self::Renderer(e) => write!(f, "surface-compatible renderer build failed: {e}"),
            Self::NoSurfaceConfig => {
                write!(
                    f,
                    "surface advertised no usable configuration for the adapter"
                )
            }
        }
    }
}

impl std::error::Error for OutputWindowError {}

/// Build the output window, surface, renderer, and blit pass on the
/// MAIN THREAD.
///
/// `app` is the Tauri manager handle from inside the `setup` callback
/// (which runs on the main thread). The returned bundle is `Send` and
/// is meant to be moved straight into
/// [`crate::render::thread::spawn`].
///
/// # Errors
///
/// See [`OutputWindowError`] — any failure aborts the R-1a output
/// bootstrap. The caller logs + skips render output (the rest of the
/// app keeps running).
pub fn build_output_window_resources<R, M>(
    app: &M,
) -> Result<RenderThreadResources<R>, OutputWindowError>
where
    R: Runtime,
    M: Manager<R>,
{
    // 1. Plain native output window — NO webview. A Tauri
    //    `WebviewWindow` hosts a `WKWebView` that paints OPAQUE over the
    //    wgpu Metal layer (an `about:blank` webview renders solid white
    //    on top of the surface, hiding every presented frame); a plain
    //    `tauri::window::Window` has no webview, so the surface's
    //    `CAMetalLayer` is the window's visible content.
    //
    //    Built VISIBLE + FOCUSED + CENTERED so the operator sees it
    //    immediately; the render thread presents the live pattern within
    //    ~16ms of the first swapchain acquire. Decorations are ON for
    //    R-1a bring-up (a titled, movable window is far easier to locate
    //    and verify than a borderless one); the borderless / fullscreen
    //    VJ-output presentation is a later slice (R-6).
    let window = WindowBuilder::new(app, OUTPUT_WINDOW_LABEL)
        .title("Chromaticity Output (R-1a test pattern)")
        .decorations(true)
        .resizable(true)
        .visible(true)
        .focused(true)
        .center()
        .inner_size(DEFAULT_LOGICAL_WIDTH, DEFAULT_LOGICAL_HEIGHT)
        .build()
        .map_err(OutputWindowError::WindowBuild)?;

    // 2. Physical size for the swapchain (inner_size on the builder is
    //    LOGICAL; the surface configures in PHYSICAL pixels).
    let physical = window.inner_size().map_err(OutputWindowError::WindowSize)?;
    let width = physical.width.max(1);
    let height = physical.height.max(1);

    // 3. Safe surface creation. `Arc<Window>` is Send + Sync +
    //    'static and implements wgpu's WindowHandle super-trait via the
    //    raw-window-handle 0.6 Arc blanket impl, so the surface is
    //    'static and wgpu keeps the window alive for the surface's
    //    lifetime. The `unsafe` raw-handle extraction is INSIDE wgpu.
    //
    //    MACOS MAIN-THREAD CONSTRAINT: this call panics if made off the
    //    main thread. We are on the main thread (the Tauri setup
    //    callback runs there); the surface is then MOVED to the render
    //    thread, but never CREATED there.
    let window = Arc::new(window);
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        backend_options: wgpu::BackendOptions::default(),
    });
    let surface: wgpu::Surface<'static> = instance
        .create_surface(Arc::clone(&window))
        .map_err(OutputWindowError::SurfaceCreate)?;

    // 4. Shell-side surface-compatible adapter for the capability /
    //    default-config reads (see module-level "Two adapters, one
    //    physical GPU"). request_adapter returns Result in wgpu 26.
    let caps_adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .map_err(|_| OutputWindowError::AdapterUnavailable)?;

    // 5. Surface-compatible renderer. CRITICAL: the renderer's device
    //    MUST come from the SAME `wgpu::Instance` that created `surface`
    //    — wgpu tracks resources in per-instance storage, so a device
    //    from a different instance panics "Surface does not exist" the
    //    moment it configures or acquires the surface on the render
    //    thread. We therefore MOVE `instance` into the renderer; it
    //    acquires its own surface-compatible adapter + device FROM that
    //    instance (also main-thread on macOS) and owns the instance for
    //    the surface's lifetime. `caps_adapter` (requested above from the
    //    same instance) keeps its own context handle and stays valid
    //    after the move, so the caps / default-config reads below still
    //    resolve against the identical Metal adapter on a single-GPU host.
    let renderer = MultipassRenderer::new_with_surface_hint(instance, &surface)
        .map_err(OutputWindowError::Renderer)?;

    // 6. Choose the swapchain format (prefer sRGB so the swapchain
    //    encodes on present) and build the surface configuration +
    //    blit pass against it.
    let caps = surface.get_capabilities(&caps_adapter);
    let format = preferred_surface_format(&caps);

    let mut config = surface
        .get_default_config(&caps_adapter, width, height)
        .ok_or(OutputWindowError::NoSurfaceConfig)?;
    config.format = format;
    config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    // Keep view_formats empty: the blit writes the surface format
    // directly (no sRGB view-format reinterpretation needed).
    config.view_formats = vec![];

    let blit = BlitPass::new(renderer.device(), format);

    Ok(RenderThreadResources {
        surface,
        config,
        renderer,
        blit,
        window,
    })
}
