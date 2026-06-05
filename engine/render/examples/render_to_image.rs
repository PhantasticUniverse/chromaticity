// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Offscreen render-to-BMP dev tool — visual self-verification of the
//! engine/render GPU pipeline WITHOUT a native window or screen capture.
//!
//! Renders a [`chromaticity_render::MultipassSpec`] headlessly via
//! [`chromaticity_render::compile_and_render_multipass`] and writes a
//! dependency-free 24-bit BMP (no `image` crate). This is the offscreen
//! counterpart to the R-1a output window: it produces a file an agent /
//! reviewer can open + judge, so a GPU render can pass the
//! "render → team-approve → human-verify" loop before anyone looks at a
//! native window.
//!
//! ```sh
//! # inline test pattern (default; no scene-adapter feature required)
//! cargo run -p chromaticity-render --example render_to_image -- /tmp/r.bmp
//!
//! # real .chroma scene through the live-spec resolver (scene-adapter feature)
//! cargo run -p chromaticity-render --example render_to_image \
//!   --features scene-adapter -- path/to/scene.chroma /tmp/r.bmp
//!
//! sips -s format png /tmp/r.bmp --out /tmp/r.png   # macOS built-in
//! ```
//!
//! Default invocation renders an inline animated UV-gradient test pattern
//! (the same shape the R-1a output window presents) to validate the
//! offscreen → image path end-to-end.
//!
//! When the first CLI arg ends in `.chroma` AND the binary was built with
//! the `scene-adapter` feature, it instead loads that scene directory
//! through the ADR-011 Phase-2 Slice R-1b live-spec builder
//! ([`chromaticity_render::MultipassSpec::from_scene_live`]) and renders
//! the resolved [`chromaticity_render::MultipassSpec`]. The render is
//! SILENT (neutral/zero audio, reactive bands at 1.0) at a fixed non-zero
//! `TIME` so time-animated scenes still show motion.

use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};

use chromaticity_render::{
    MultipassSpec, PassDef, TargetFormat, UniformInput, UniformValue, compile_and_render_multipass,
};

/// Inline single-pass animated UV-gradient fragment (original; mirrors the
/// R-1a output-window test pattern). One scalar `u_time` uniform at
/// `set = 0, binding = 0`, matching the engine's single-uniform fixture.
const TEST_PATTERN_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
layout(set = 0, binding = 0) uniform Uniforms {
    float u_time;
};
void main() {
    float band = 0.5 + 0.5 * sin((uv.x + uv.y) * 6.2831853 + u_time * 2.0);
    fragColor = vec4(uv.x, uv.y, band, 1.0);
}
"#;

fn test_pattern_spec(width: u32, height: u32, time_secs: f32) -> MultipassSpec {
    MultipassSpec {
        passes: vec![PassDef {
            label: Some("render_to_image/test-pattern".to_string()),
            fragment_glsl: TEST_PATTERN_FRAGMENT.to_string(),
            uniforms: vec![UniformInput {
                name: "u_time".to_string(),
                value: UniformValue::Scalar(time_secs),
            }],
            input_textures: Vec::new(),
            target_format: TargetFormat::Rgba8Unorm,
            feedback_self: false,
        }],
        target_size: (width.max(1), height.max(1)),
        frame_count: 1,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let arg1 = std::env::args().nth(1);

    // Scene mode: first arg is a `.chroma` directory, second is the output
    // `.bmp`. Gated behind the `scene-adapter` feature so the default-
    // feature build still compiles the inline test-pattern path only.
    #[cfg(feature = "scene-adapter")]
    if let Some(scene_arg) = arg1.as_deref() {
        if scene_arg.ends_with(".chroma") {
            let out_path = std::env::args()
                .nth(2)
                .ok_or("scene mode requires an output .bmp path as the second argument")?;
            return render_scene(scene_arg, &out_path);
        }
    }

    let out_path = arg1.unwrap_or_else(|| "render-out.bmp".to_string());
    let width: u32 = 640;
    let height: u32 = 360;
    let time_secs: f32 = 1.0;

    let spec = test_pattern_spec(width, height, time_secs);
    let rgba = compile_and_render_multipass(&spec)?;

    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        return Err(format!(
            "render produced {} bytes, expected {expected} ({width}x{height} RGBA8)",
            rgba.len()
        )
        .into());
    }

    write_bmp_24(&out_path, width, height, &rgba)?;
    println!(
        "wrote {out_path} ({width}x{height}); view: sips -s format png {out_path} --out {out_path}.png"
    );
    Ok(())
}

/// Render a real `.chroma` scene directory through the ADR-011 Phase-2
/// Slice R-1b live-spec builder and write the result as a 24-bit BMP.
///
/// Loads `scene.json` via [`chromaticity_scene::loader::load_chroma_dir`],
/// reads the sibling `main.glsl`, resolves the scene's `UNIFORMS` against a
/// SILENT [`LiveUniforms`] at a fixed non-zero `TIME`, and rasterises the
/// single-pass [`MultipassSpec`] at `1280x720` (the scene carries no
/// `WIDTH` / `HEIGHT`, so the adapter default target size is the render
/// size).
#[cfg(feature = "scene-adapter")]
fn render_scene(scene_dir: &str, out_path: &str) -> Result<(), Box<dyn Error>> {
    use std::path::Path;

    use chromaticity_render::scene_adapter::{LiveUniforms, SceneAdapterOptions};
    use chromaticity_scene::loader::load_chroma_dir;

    const RENDER_SIZE: (u32, u32) = (1280, 720);

    let dir = Path::new(scene_dir);
    let scene = load_chroma_dir(dir)?;
    let main_glsl = std::fs::read_to_string(dir.join("main.glsl"))?;

    let options = SceneAdapterOptions {
        fragment_sources: vec![Some(main_glsl)],
        default_target_size: RENDER_SIZE,
        ..SceneAdapterOptions::default()
    };
    let live = LiveUniforms::silent(1.0, (RENDER_SIZE.0 as f32, RENDER_SIZE.1 as f32));

    let spec = MultipassSpec::from_scene_live(&scene, &options, &live)?;
    let (width, height) = spec.target_size;
    let rgba = compile_and_render_multipass(&spec)?;

    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        return Err(format!(
            "render produced {} bytes, expected {expected} ({width}x{height} RGBA8)",
            rgba.len()
        )
        .into());
    }

    write_bmp_24(out_path, width, height, &rgba)?;
    println!(
        "wrote {out_path} ({width}x{height}) from scene {scene_dir}; \
         view: sips -s format png {out_path} --out {out_path}.png"
    );
    Ok(())
}

/// Write `rgba` (row-major, top-to-bottom, RGBA8) as a 24-bit BMP
/// (bottom-up, BGR, rows zero-padded to a 4-byte boundary). No external
/// crates — BMP is the simplest universally-readable raster container.
fn write_bmp_24(path: &str, width: u32, height: u32, rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    let w = width as usize;
    let h = height as usize;
    let row_unpadded = w * 3;
    let pad = (4 - (row_unpadded % 4)) % 4;
    let row_size = row_unpadded + pad;
    let pixel_bytes = row_size * h;
    let file_size = 54usize + pixel_bytes;

    let file = File::create(path)?;
    let mut out = BufWriter::new(file);

    // BITMAPFILEHEADER (14 bytes).
    out.write_all(b"BM")?;
    out.write_all(&u32::try_from(file_size)?.to_le_bytes())?;
    out.write_all(&0u32.to_le_bytes())?; // reserved
    out.write_all(&54u32.to_le_bytes())?; // pixel-data offset
    // BITMAPINFOHEADER (40 bytes).
    out.write_all(&40u32.to_le_bytes())?;
    out.write_all(&i32::try_from(width)?.to_le_bytes())?;
    out.write_all(&i32::try_from(height)?.to_le_bytes())?; // positive => bottom-up
    out.write_all(&1u16.to_le_bytes())?; // planes
    out.write_all(&24u16.to_le_bytes())?; // bits per pixel
    out.write_all(&0u32.to_le_bytes())?; // BI_RGB (no compression)
    out.write_all(&u32::try_from(pixel_bytes)?.to_le_bytes())?;
    out.write_all(&2835i32.to_le_bytes())?; // x px/m (~72 dpi)
    out.write_all(&2835i32.to_le_bytes())?; // y px/m
    out.write_all(&0u32.to_le_bytes())?; // palette colors
    out.write_all(&0u32.to_le_bytes())?; // important colors

    // BMP scanlines run bottom-to-top; our RGBA buffer is top-to-bottom.
    let pad_zeros = [0u8; 3];
    for y in (0..h).rev() {
        let row = y * w * 4;
        for x in 0..w {
            let i = row + x * 4;
            // Bounds already validated by the caller; `get` keeps this
            // panic-free regardless (workspace forbids unwrap/expect).
            let r = rgba.get(i).copied().unwrap_or(0);
            let g = rgba.get(i + 1).copied().unwrap_or(0);
            let b = rgba.get(i + 2).copied().unwrap_or(0);
            out.write_all(&[b, g, r])?; // BMP stores BGR
        }
        if pad > 0 {
            out.write_all(&pad_zeros[..pad])?;
        }
    }
    out.flush()?;
    Ok(())
}
