# CODEBUDDY.md This file provides guidance to CodeBuddy when working with code in this repository.

## Build & Development Commands

**Build the library:**
```bash
cargo build
```

**Run the minimal example (a video player with pause, loop, and seek controls):**
```bash
cargo run --example minimal
```
Requires a test video at `.media/test.mp4` (excluded from the crate package by `Cargo.toml`).

**Build in release mode:**
```bash
cargo build --release
```

**Check code without producing binaries:**
```bash
cargo check
```

**Nix development shell (Linux only, includes GStreamer plugins and Vulkan):**
```bash
nix develop
```
The flake targets only `x86_64-linux`. Non-Nix users must install GStreamer runtime manually following [gstreamer-rs installation instructions](https://github.com/sdroege/gstreamer-rs#installation).

## Architecture Overview

This is a video player widget for the [Iced](https://iced.rs/) GUI framework, built on GStreamer. The crate provides two public types: `Video` (the media handle) and `VideoPlayer` (the Iced widget). Video frames are decoded via GStreamer's `playbin` pipeline, output as NV12 raw video into an `appsink`, and rendered on the GPU through a custom wgpu pipeline with a WGSL shader performing YUV-to-RGB color conversion.

### Core Module (`src/video.rs`)

`Video` is the central media handle. Its constructor `Video::new(uri: &url::Url)` calls `gst::init()`, then constructs a GStreamer pipeline string:
```
playbin uri="..." text-sink="appsink name=iced_text sync=true drop=true" video-sink="videoscale ! videoconvert ! appsink name=iced_video drop=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1"
```
This uses GStreamer's `playbin` element which auto-negotiates demuxing and decoding for any format GStreamer supports (local files or network streams). The video branch forces NV12 output with square pixels.

After setting the pipeline to `Playing` state, the constructor extracts resolution, framerate, and duration from the negotiated caps. A background worker thread is spawned that runs for the lifetime of the `Video`. This thread continuously pulls samples from the `appsink` (using `try_pull_sample` or `try_pull_preroll` with a 16ms timeout) and writes them into a shared `Arc<Mutex<Frame>>`. An `AtomicBool` (`upload_frame`) signals the render path that new data is available. The thread also handles subtitle text from a second optional `appsink` named `iced_text`.

`Internal` is the inner struct behind `RwLock<Internal>` inside `Video`, holding the pipeline reference, bus, frame data, playback state (speed, looping, EOS flag, AV sync), and subtitle state.

`Video` exposes programmatic control: `set_paused()`, `seek()`, `set_speed()`, `set_volume()`, `set_muted()`, `set_looping()`, `restart_stream()`, `step_one_frame()`, `set_subtitle_url()`, and read accessors for position, duration, size, framerate. The `thumbnails()` method seeks to multiple positions, captures frames, and converts them to RGBA `image::Handle` values via CPU-side YUV-to-RGBA conversion for use as Iced image handles.

AV sync is implemented by tracking the latency between frame arrival and rendering (the time delta between the worker thread timestamping a frame and the `draw()` method consuming it). A running average of this offset is periodically applied to the pipeline's `av-offset` property to keep audio in sync with the displayed video.

### Widget Module (`src/video_player.rs`)

`VideoPlayer<'a, Message, Theme, Renderer>` implements Iced's `Widget` trait. It holds a reference to a `Video` and callbacks for `on_end_of_stream`, `on_new_frame`, `on_subtitle_text`, and `on_error`.

The widget's `update()` method is driven by Iced's `RedrawRequested` window event. On each redraw, it drains the GStreamer bus for `Error` and `Eos` messages. On EOS, if looping is enabled, it restarts the stream via `seek(0)`; otherwise it pauses and sets the `is_eos` flag. If a new frame was uploaded, it fires `on_new_frame`. Subtitle text changes fire `on_subtitle_text`. It calls `shell.request_redraw()` to maintain continuous rendering (or `request_redraw_at` with a 32ms delay when paused/EOS to avoid busy-waiting).

The `draw()` method determines the bounds based on `content_fit` (like Iced's `Image` widget), marks the frame as consumed by swapping `upload_frame` to `false`, computes the AV sync offset, and then submits a `VideoPrimitive` to the renderer via `renderer.draw_primitive()`. If the video extends beyond the clip bounds, it uses `renderer.with_layer()` for clipping.

`layout()` mirrors Iced's `Image::layout` — it resolves the video's native resolution against the widget's width/height `Length` values and the `ContentFit` (Contain, Cover, Fill, None, ScaleDown).

### GPU Pipeline Module (`src/pipeline.rs`)

`VideoPipeline` implements `iced_wgpu::primitive::Pipeline`, the trait that allows custom rendering primitives in Iced's wgpu backend. It manages a `BTreeMap<u64, VideoEntry>` keyed by `video_id` (a monotonically increasing atomic counter assigned per `Video` instance), enabling multiple video players in the same application.

On construction (`Pipeline::new`), it creates:
- A WGSL shader module from `shader.wgsl`
- A bind group layout with four bindings: Y-plane texture (R8Unorm), UV-plane texture (Rg8Unorm), a sampler, and a dynamic uniform buffer for quad position
- A render pipeline with a full-screen triangle-strip vertex shader (no vertex buffers; vertex index drives UV generation)
- A linear-filtering clamp-to-edge sampler

`VideoPrimitive` is the custom primitive type. Its `prepare()` method is called by Iced's renderer each frame. When `upload_frame` is true, it locks the shared frame mutex, reads the raw NV12 buffer and stride from GStreamer's `VideoMeta`, and calls `VideoPipeline::upload()`.

`upload()` performs a lazy initialization pattern: on first upload for a given `video_id`, it creates two wgpu textures (Y as `R8Unorm` at native resolution, UV as `Rg8Unorm` at half resolution), a uniform buffer (256 slots for up to 256 concurrent video players), and a bind group. Then it writes Y-plane data using `queue.write_texture()` with `bytes_per_row` set to the actual stride (which may differ from width due to hardware alignment) and UV-plane data from the interleaved offset. This is a critical detail — the stride from `VideoMeta::stride()[0]` must be respected, as GPU texture uploads need correct row pitch.

`prepare()` also writes the quad's screen-space rectangle into the dynamic uniform buffer slot and increments the instance counter. `draw()` issues a render pass that binds the pipeline, bind group (with dynamic offset into the uniform buffer), sets scissor, and draws 6 vertices (two triangles) to render the textured quad.

`trim()` is called periodically by Iced to clean up entries whose `alive` flag is false, destroying their GPU resources.

### WGSL Shader (`src/shader.wgsl`)

The vertex shader generates a full-screen quad from `uniforms.rect` (a `vec4<f32>` of `(x1, y1, x2, y2)`) using only the vertex index — six hardcoded vertices form two triangles. UV coordinates `(0,0)` through `(1,1)` are passed to the fragment shader.

The fragment shader implements BT.709 YUV-to-RGB conversion using the standard formula:
- Normalizes Y from limited range (16-235) and UV from limited range (16-240)
- Samples the Y texture (`.r` channel) and UV texture (`.r` for U, `.g` for V)
- Applies the BT.709 matrix multiplication
- Clamps output to `[0,1]` range with full alpha

### Error Handling (`src/lib.rs`)

A unified `Error` enum using `thiserror` wraps GStreamer errors (`glib::Error`, `glib::BoolError`, `StateChangeError`), I/O errors, and domain-specific errors (missing bus, missing appsink, cast failure, invalid URI, missing caps, invalid framerate, sync/lock failures).

### Example (`examples/minimal.rs`)

A full video player app demonstrating pause/play toggle, looping toggle, seek bar with scrubbing, and time display. It loads a test video from `.media/test.mp4`, wires up `EndOfStream` and `NewFrame` callbacks, and uses `VideoPlayer` with `ContentFit::Contain`.

### Key Dependencies

- **iced 0.14** with `image`, `advanced`, and `wgpu` features — the GUI framework and its wgpu rendering backend
- **gstreamer 0.23** / **gstreamer-app 0.23** / **gstreamer-video 0.23** / **gstreamer-base 0.23** — the multimedia framework for decoding and frame extraction
- **glib 0.20** — GObject type system bindings
- **url 2** — URI parsing for media sources

### Platform Notes

GStreamer must be installed as a system dependency. On Linux, use the system package manager or the Nix flake. On Windows (MSVC/MinGW) and macOS, follow the gstreamer-rs installation guide. The project uses Rust edition 2024 with resolver 2.
