# iced_video_player — Feature Specification

**Version**: 1.0.0 (reverse-engineered from existing codebase)
**Generated**: 2026-07-23
**Source**: Codebase analysis of crate v0.6.0 + bundled desktop application

---

## 1. Project Overview

`iced_video_player` is a dual-purpose Rust project:

1. **A library crate** providing a GPU-accelerated video player widget for the
   [Iced](https://iced.rs/) GUI framework, with GStreamer as the media backend.
2. **A desktop video player application** that exercises the library and adds a
   rich keyboard-driven UI with subtitle support, dictionary lookup, and
   settings persistence.

The video pipeline decodes media via GStreamer's `playbin`, outputs NV12 raw
video into `appsink`, and renders frames through a custom wgpu pipeline with a
WGSL shader performing BT.709 YUV→RGB color conversion on the GPU.

**Target platforms**: Linux, Windows (MSVC/MinGW), macOS.

---

## 2. User Stories

### 2.1 Library Crate

#### US-01 — Play a video from a URI

> As a Rust/Iced application developer, I want to create a `Video` from a local
> file path or network URI so that my application can play media content.

**Acceptance criteria**:
- `Video::new(&url::Url)` returns a valid `Video` handle for any URI GStreamer
  can handle (local files, HTTP streams, etc.).
- The GStreamer pipeline starts automatically in `Playing` state.
- Video resolution, framerate, and duration are extracted from the negotiated
  caps and exposed via accessor methods.
- Errors from GStreamer (missing codecs, invalid URI, etc.) are returned via
  the unified `Error` enum.

#### US-02 — Render video frames on GPU

> As an application developer, I want the `VideoPlayer` widget to render video
> frames with hardware acceleration so that playback is smooth and efficient.

**Acceptance criteria**:
- The `VideoPlayer` widget implements Iced's `Widget` trait.
- Frames are uploaded as NV12 textures (Y plane as `R8Unorm`, UV plane as
  `Rg8Unorm`) respecting the GStreamer `VideoMeta` stride.
- YUV→RGB conversion executes on GPU via a WGSL fragment shader using the
  BT.709 standard.
- `ContentFit` (Contain, Cover, Fill, None, ScaleDown) works identically to
  Iced's `Image` widget.
- Continuous redraw is maintained during playback; paused/EOS state requests
  redraw at 32ms intervals to avoid busy-waiting.

#### US-03 — Control playback programmatically

> As an application developer, I want to call methods on `Video` to pause,
> seek, change speed, adjust volume, and enable looping, so I can build custom
> player controls.

**Acceptance criteria**:
- `set_paused(bool)` toggles between Playing and Paused states.
- `seek(time)` and `seek_f32(fraction 0.0–1.0)` reposition the playhead.
- `set_speed(f64)` changes playback rate.
- `set_volume(f64)`, `set_muted(bool)` adjust audio.
- `set_looping(bool)` enables/disables automatic restart on end-of-stream.
- `restart_stream()` seeks to position 0.
- `step_one_frame(forward)` advances by exactly one frame (forward or backward).
- Getters: `position()`, `duration()`, `size()`, `framerate()`, `id()`.

#### US-04 — AV synchronization

> As an application developer, I want video and audio to stay in sync without
> manual configuration.

**Acceptance criteria**:
- A running average of the latency between frame arrival (worker thread
  timestamp) and frame rendering (`draw()` timestamp) is computed.
- This latency offset is periodically applied to the pipeline's `av-offset`
  property.
- Drift stays within human-perceptible limits under normal playback.

#### US-05 — Generate video thumbnails

> As an application developer, I want to capture frames at specific positions
> to generate preview thumbnails.

**Acceptance criteria**:
- `thumbnails(positions: &[f64])` seeks to each given position, captures one
  frame, converts it from NV12 to RGBA on CPU, and returns an `image::Handle`.
- The original playback position is restored after capture.
- Missing positions (past duration, seek failure) return empty handles.

#### US-06 — Handle errors gracefully

> As an application developer, I want errors surfaced through callbacks so I
> can display them to users without the widget panicking.

**Acceptance criteria**:
- GStreamer bus errors are drained in `update()` and forwarded to `on_error`.
- End-of-stream fires `on_end_of_stream`; if looping is enabled, the stream
  restarts automatically.
- Worker thread timeouts and disconnections return `FlowError` rather than
  panicking.
- Lock poisoning on the shared `Frame` mutex is caught and converted to a
  recoverable error.

#### US-07 — Render bitmap subtitles (PGS)

> As an application developer, I want PGS (Blu-ray) subtitles decoded on CPU
> and uploaded to the GPU overlay so that embedded bitmap subtitles are visible.

**Acceptance criteria**:
- A pure-Rust PGS decoder handles RLE decompression, palette parsing, and
  YUV→RGBA conversion.
- Both `.sup` format (13-byte segment headers with PTS) and raw GStreamer/
  matroskademux format (3-byte segment headers) are supported.
- Decoded `PgsImage` is uploaded as an RGBA texture and composited onto the
  video frame in the shader.
- Subtitle timing (auto-clear on expiry) is handled by the worker thread.

#### US-08 — Multiple concurrent video instances

> As an application developer, I want to render multiple video players in the
> same Iced application without resource conflicts.

**Acceptance criteria**:
- Each `Video` gets a unique monotonically increasing `u64` ID.
- The `VideoPipeline` manages per-video wgpu resources in a `BTreeMap`.
- `trim()` cleans up GPU resources for dropped videos.
- Dynamic uniform buffer offsets support up to 256 concurrent instances.

### 2.2 Desktop Application

#### US-09 — Open video files with drag-and-drop

> As a desktop user, I want to open video files through the application so I
> can watch my local media.

**Acceptance criteria**:
- File open dialog launches via `rfd::FileDialog`, filtered to common video
  extensions.
- File path passed as command-line argument or dropped onto the window opens
  directly.
- Replacing the current video stops the old pipeline and creates a new one.

#### US-10 — Navigate playback with keyboard shortcuts

> As a desktop user, I want comprehensive keyboard shortcuts so I can control
> playback without touching the mouse.

**Acceptance criteria**:

| Key | Action |
|-----|--------|
| Space / K | Toggle pause |
| ← / → | Seek ±5s |
| Ctrl+← / Ctrl+→ | Seek ±1s |
| Shift+← / Shift+→ | Seek ±30s |
| ↑ / ↓ | Volume ±5% |
| Ctrl+↑ / Ctrl+↓ (or `[`/`]`) | Speed ±0.25x (range 0.25x–4x) |
| M | Toggle mute |
| F / F11 | Toggle fullscreen |
| Esc | Exit fullscreen / close dictionary |
| R | Restart stream |
| L | Toggle loop |
| O | Open file dialog |
| S | Cycle content fit (Contain → Cover → Fill) |
| , (comma) / . (period) | Frame step backward / forward |

#### US-11 — Display and search subtitles

> As a desktop user, I want subtitles displayed on screen and clickable for
> dictionary lookup so I can understand foreign-language content.

**Acceptance criteria**:

##### 11a — Auto-discovery
- On file open, external subtitles in the video's directory matching the
  video's stem are searched (extensions: `.srt`, `.ass`, `.ssa`, `.vtt`,
  `.sub`, `.smi`).
- Preference order: no-language-suffix, then English suffix (`.en`, `.eng`,
  `.en-US`), then first candidate.

##### 11b — Manual loading
- `S` key opens a file dialog for manual subtitle file selection.

##### 11c — Embedded subtitle extraction
- `.sup` extraction via ffmpeg (`copy` codec, to temp file).
- PGS bitmap decoded by the crate's built-in decoder.
- OCR via Windows.Media.Ocr (Windows 10+) with 3x upscaling for accuracy.
- Output written as `.srt` next to the video file; reused if already present.

##### 11d — Rendering
- Text subtitles are cleaned (HTML tags stripped, XML entities replaced, `\N`
  and `\n` converted to newlines).
- Subtitles render with configurable font size (12–48px, ±2px step) on a
  semi-transparent dark background.
- Individual words in the subtitle are clickable (`MouseArea` + pointer
  cursor), firing `Message::SearchWord`.

#### US-12 — Dictionary lookup

> As a desktop user, I want to click any subtitle word and see its definition
> so I can learn vocabulary while watching.

**Acceptance criteria**:
- Clicking a word triggers `Message::SearchWord`, spawning async lookups.
- **Chinese translation**: MyMemory API (`https://api.mymemory.translated.net`),
  `en|zh-CN` langpair; spurious "MYMEMORY WARNING" responses are stripped.
- **English definitions**: dictionaryapi.dev; phonetics, part-of-speech
  groupings, and example sentences extracted.
- `DictResult` aggregates both sources; error message shown if neither returns
  results.
- Dictionary panel opens in the sidebar with tab UI.

#### US-13 — Sidebar UI

> As a desktop user, I want a sidebar with tabs for subtitles and dictionary
> so I can manage these features without cluttering the video area.

**Acceptance criteria**:
- Sidebar is a resizable panel to the right of the video.
- Tabs: "Subtitles" (shows current subtitle text) and "Dictionary" (shows
  lookup results).
- Subtitle tab displays the active subtitle with all clickable words.
- Dictionary tab shows: Chinese translation (prominent), phonetic/IPA,
  part-of-speech sections with definitions, and example sentences.

#### US-14 — Persistent settings and history

> As a desktop user, I want the application to remember my preferences and
> recently opened files across sessions.

**Acceptance criteria**:
- Settings stored as JSON at platform-appropriate paths:
  - Windows: `%APPDATA%\video-player\settings.json`
  - Linux/macOS: `$HOME/.config/video-player/settings.json`
- Persisted fields: `subtitle_font_size` (12–48), `history_enabled`,
  `max_history_items` (10–1000), `recent_files` (deduplicated, most-recent-first).
- Read/write errors are silently swallowed (best-effort persistence).
- Settings auto-saved on change.

#### US-15 — Fullscreen support

> As a desktop user, I want to toggle fullscreen mode so I can watch videos
> without distractions.

**Acceptance criteria**:
- `F` or `F11` toggles fullscreen; `Esc` exits fullscreen.
- In fullscreen mode, the window has no decorations.
- Content fit and all keyboard shortcuts continue to work.

---

## 3. Key Data Structures & API

### 3.1 `Video` (public handle)

```rust
pub struct Video { /* internal: Arc<RwLock<Internal>> + atomic state */ }

impl Video {
    pub fn new(uri: &url::Url) -> Result<Self, Error>;
    pub fn id(&self) -> u64;
    pub fn close(&mut self);

    // Playback control
    pub fn set_paused(&self, paused: bool);
    pub fn toggle_pause(&self);
    pub fn seek(&self, time: gst::ClockTime);
    pub fn seek_f32(&self, pos: f32);
    pub fn set_speed(&self, speed: f64);
    pub fn set_volume(&self, volume: f64);
    pub fn set_muted(&self, muted: bool);
    pub fn set_looping(&self, looping: bool);
    pub fn restart_stream(&self);
    pub fn step_one_frame(&self, forward: bool);
    pub fn step_one_frame_fwd(&self);       // convenience alias

    // Subtitle management
    pub fn set_subtitle_url(&self, url: Option<&url::Url>);
    pub fn set_subtitle_track(&self, index: i32);

    // Metadata queries
    pub fn position(&self) -> Option<gst::ClockTime>;
    pub fn duration(&self) -> Option<gst::ClockTime>;
    pub fn size(&self) -> iced::Size<u32>;
    pub fn framerate(&self) -> iced_wgpu::core::Fraction;

    // Thumbnail generation
    pub fn thumbnails(&self, positions: &[f64]) -> Vec<image::Handle>;
}
```

### 3.2 `VideoPlayer` (Iced widget)

```rust
pub struct VideoPlayer<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    video: &'a Video,
    content_fit: ContentFit,
    show_subtitle: bool,          // GUI-level toggle
    subtitle_font_size: f32,
    on_end_of_stream: Option<Message>,
    on_new_frame: Option<Message>,
    on_subtitle_text: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_subtitle_image: Option<Box<dyn Fn(Option<super::pgs::PgsImage>) -> Message + 'a>>,
    on_error: Option<Box<dyn Fn(Error) -> Message + 'a>>,
}

impl<'a, M, T, R> VideoPlayer<'a, M, T, R> {
    pub fn new(video: &'a Video) -> Self;
    pub fn content_fit(self, fit: ContentFit) -> Self;
    pub fn show_subtitle(self, show: bool) -> Self;
    pub fn subtitle_font_size(self, size: f32) -> Self;
    pub fn on_end_of_stream(self, msg: M) -> Self;
    pub fn on_new_frame(self, msg: M) -> Self;
    pub fn on_subtitle_text<F: Fn(String) -> M + 'a>(self, f: F) -> Self;
    pub fn on_subtitle_image<F: Fn(Option<super::pgs::PgsImage>) -> M + 'a>(self, f: F) -> Self;
    pub fn on_error<F: Fn(Error) -> M + 'a>(self, f: F) -> Self;
}
```

### 3.3 GPU Pipeline

```rust
// Custom primitive submitted by VideoPlayer::draw()
pub struct VideoPrimitive {
    pub video_id: u64,
    pub bounds: iced::Rectangle,
    pub clip_bounds: iced::Rectangle,
    pub upload_frame: Arc<AtomicBool>,
    pub frame: Arc<Mutex<Frame>>,       // Frame = gst::Sample (newtype)
    pub last_frame_time: Arc<Mutex<Instant>>,
    pub show_subtitle: bool,
    pub subtitle_image: Arc<Mutex<Option<PgsImage>>>,
    pub upload_subtitle: Arc<AtomicBool>,
}

// Pipeline registered via iced_wgpu::primitive::Pipeline trait
pub struct VideoPipeline {
    entries: BTreeMap<u64, VideoEntry>,   // per-video resources
    next_entry_id: u64,                   // dynamic uniform buffer index
}

struct VideoEntry {
    y_texture: wgpu::Texture,
    uv_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    alive: bool,
    // Subtitle overlay
    sub_texture: Option<wgpu::Texture>,
    has_sub: bool,
}
```

### 3.4 PGS Decoder

```rust
pub struct PgsImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub x: u32,              // X offset within video frame
    pub y: u32,              // Y offset within video frame
    pub frame_width: u32,    // authored video frame width
    pub frame_height: u32,   // authored video frame height
}

pub struct PgsDisplaySet {
    pub pts_seconds: f64,
    pub image: Option<PgsImage>,  // None = clear set (subtitle end)
}

pub fn decode(data: &[u8]) -> Option<PgsImage>;
pub fn parse_sup(data: &[u8]) -> Vec<PgsDisplaySet>;
```

### 3.5 Error Enum

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error("GStreamer error: {0}")]             GStreamer(#[from] glib::Error),
    #[error("Bool error: {0}")]                   BoolError(#[from] glib::BoolError),
    #[error("State change error: {0}")]           StateChangeError(#[from] gst::StateChangeError),
    #[error("I/O error: {0}")]                    Io(#[from] std::io::Error),
    #[error("Missing pipeline bus")]              MissingBus,
    #[error("Missing appsink: {0}")]              MissingAppSink(String),
    #[error("Cast error")]                        Cast,
    #[error("Invalid URI: {0}")]                  InvalidUri(String),
    #[error("Missing caps")]                      MissingCaps,
    #[error("Invalid framerate")]                 InvalidFramerate,
    #[error("Sync/lock error")]                   SyncError,
    #[error("Playback error: {0}")]               Playback(String),
}
```

### 3.6 Application Settings

```rust
pub struct AppSettings {
    pub subtitle_font_size: f32,       // 12.0–48.0, step 2.0
    pub history_enabled: bool,         // default: true
    pub max_history_items: usize,      // 10–1000, step 10
    pub recent_files: Vec<String>,     // most-recent first, deduplicated
}
```

---

## 4. Non-Functional Requirements

### NFR-01 — Performance
- Frame decoding and GPU upload MUST NOT block the Iced event loop.
- Worker thread pulls samples with 16ms timeout.
- GPU uploads use `queue.write_texture()` with correct stride, no
  intermediate CPU copies beyond the GStreamer buffer.

### NFR-02 — Cross-Platform
- Code MUST compile on Linux, Windows (MSVC/MinGW), macOS.
- Platform-specific code (Windows OCR, subtitle extraction) MUST be gated by
  `#[cfg]` attributes.
- Nix flake provides hermetic Linux build but MUST NOT be required.

### NFR-03 — Error Resilience
- Worker thread MUST NOT panic on pipeline disconnection, EOS, or timeout.
- Lock poisoning on the shared `Frame` mutex is caught and surfaced.
- Settings IO failures are silently swallowed (non-critical).

### NFR-04 — Memory
- GPU textures for dropped `Video` instances MUST be cleaned up via `trim()`.
- Static uniform buffer holds up to 256 entries; exceeding this is a
  hard limit.
- Thumbnail frame capture temporarily seeks the pipeline; position MUST
  be restored on completion.

### NFR-05 — Shader Correctness
- The WGSL fragment shader MUST implement BT.709 YUV→RGB conversion:
  - Y normalized from limited range (16–235)
  - UV normalized from limited range (16–240)
  - Standard BT.709 matrix applied
  - Output clamped to `[0, 1]`

---

## 5. Architecture Diagram

```
┌──────────────────────────────────────────────────────┐
│                    Iced Application                    │
│  ┌──────────┐  ┌──────────┐  ┌─────────────────┐    │
│  │  Video   │  │VideoPlayer│  │  Sidebar (Dict/ │    │
│  │ (handle) │  │ (widget)  │  │  Subtitles)     │    │
│  └────┬─────┘  └─────┬────┘  └─────────────────┘    │
└───────┼──────────────┼───────────────────────────────┘
        │              │
        │     VideoPrimitive (custom wgpu primitive)
        │              │
        │    ┌─────────▼──────────┐
        │    │   VideoPipeline    │
        │    │ (iced_wgpu backend)│
        │    │ ┌───────────────┐  │
        │    │ │ WGSL Shader   │  │
        │    │ │ YUV→RGB (GPU) │  │
        │    │ └───────────────┘  │
        │    └────────────────────┘
        │
   ┌────▼──────────────────────┐
   │     GStreamer Pipeline     │
   │  playbin                   │
   │  ├── video → videoscale    │
   │  │        → videoconvert   │
   │  │        → appsink(NV12)  │──► Worker Thread ──► Frame Buffer
   │  └── text  → appsink       │──► Subtitle Buffer
   └────────────────────────────┘
```

---

## 6. Constitution Compliance

| Principle | Status | Notes |
|-----------|--------|-------|
| I. GPU-First Rendering | ✅ | YUV→RGB on GPU via WGSL; CPU fallback only for thumbnails |
| II. GStreamer Backend | ✅ | `playbin` pipeline; no alternative backends |
| III. Widget Composability | ✅ | `VideoPlayer` is pure Iced `Widget`; no bundled GUI controls |
| IV. Error Resilience | ✅ | Unified `Error` enum; worker thread never panics |
| V. Cross-Platform | ✅ | Platform code gated; Nix is optional |

---

## 7. Dependencies (Key)

| Crate | Version | Purpose |
|-------|---------|---------|
| iced | 0.14 | GUI framework (widget, wgpu, image) |
| gstreamer | 0.23 | Media pipeline |
| gstreamer-app | 0.23 | `appsink` for frame/subtitle extraction |
| gstreamer-video | 0.23 | `VideoMeta` for stride/format |
| glib | 0.20 | GObject type system |
| url | 2 | URI parsing |
| thiserror | 2 | Error derive |
| serde / serde_json | 1 | Settings JSON persistence |
| image | 0.25 | CPU-side RGBA conversion (thumbnails, PGS) |
| ureq | 3 | HTTP client for dictionary APIs |
| rfd | 0.15 | Native file dialogs |
| windows | 0.61 | Windows.Media.Ocr for PGS→text (Windows only) |
| log | 0.4 | Logging facade |
