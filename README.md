[English](./README.md) | [中文](./README.zh-CN.md)

# ELP11

A full-featured, keyboard-driven desktop video player built with [Iced](https://iced.rs/) and [GStreamer](https://gstreamer.freedesktop.org/). It ships both a reusable GPU-accelerated video widget library (`iced_video_player`) and a desktop application that exercises it with subtitle support, an embedded dictionary, a playlist, and persistent settings.

<img src=".media/screenshot.png" width="70%" />

## Features

### Playback
- Plays **anything GStreamer's `playbin` supports** — local files and network streams.
- GPU rendering via a custom wgpu pipeline: frames are decoded to NV12 and converted to RGB on the GPU with a WGSL BT.709 YUV→RGB shader.
- Automatic AV synchronization (running-average latency offset applied to the pipeline).
- Frame-stepping (forward and backward), variable speed (0.25×–4×), volume, mute, and loop.
- `ContentFit` modes (Contain / Cover / Fill / None / ScaleDown), cycled with a single key.
- Fullscreen toggle.
- Thumbnail capture at arbitrary timestamps (CPU YUV→RGBA fallback for image handles).

### Subtitles
- **Auto-discovery** of external subtitles next to the video (`.srt`, `.ass`, `.ssa`, `.vtt`, `.sub`, `.smi`), preferring the no-language / English variant.
- **Manual loading** via a file dialog.
- **Embedded subtitle extraction**:
  - Text streams (SRT/ASS/VTT) are converted directly with `ffmpeg`.
  - PGS (Blu-ray) bitmap subtitles are decoded by the built-in pure-Rust PGS decoder and OCR'd to SRT using `Windows.Media.Ocr` (Windows 10+) with 3× upscaling for accuracy. The resulting `.srt` is cached next to the video.
- Text subtitles are cleaned (HTML tags stripped, entities resolved) and every word is **clickable for dictionary lookup**.
- `Home` / `End` keys jump between subtitle cues; a rapid double-press of `Home` steps back one extra cue.

### Dictionary
- Clicking any subtitle word opens a **Youdao dictionary** panel embedded as a native WebView (`wry`) over the sidebar. The page loads once and subsequent searches are performed via JavaScript injection — no reload — so lookups are instant.
- A mobile user agent and an injected dark-mode style sheet (via `darkreader.js`) keep the embedded page readable and on-theme.
- (A fallback API path — MyMemory for Chinese translations and dictionaryapi.dev for English definitions — is also included.)

### Playlist
- Drag-and-drop files or a folder onto the window to build a playlist.
- Opening a single video auto-populates the playlist with the other videos in its directory.
- `PageUp` / `PageDown` (or the on-screen controls) move to the previous / next item.

### Settings & Resume
- Settings are persisted as JSON:
  - Windows: `%APPDATA%\ELP11\settings.json`
  - Linux/macOS: `$HOME/.config/ELP11/settings.json`
- Configurable subtitle font size (12–48 px), history toggle, and max recent-items count.
- **Crash-resilient resume**: the playback position is auto-saved every few seconds and on close, so a crash never loses more than a few seconds of progress. Reopening a file seeks back to where you left off (unless the file was within 10 s of the end, in which case it starts fresh).
- Settings are written atomically (temp file + rename) to survive mid-write crashes.
- Recently opened files are tracked and re-openable from the Settings tab.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Space / K | Toggle pause |
| ← / → | Seek ±5 s |
| Ctrl+← / Ctrl+→ | Seek ±1 s |
| Shift+← / Shift+→ | Seek ±30 s |
| ↑ / ↓ | Volume ±5 % |
| Ctrl+↑ / Ctrl+↓ (or `[` / `]`) | Speed ±0.25× (0.25×–4×) |
| M | Toggle mute |
| F / F11 / Enter | Toggle fullscreen |
| Esc | Exit fullscreen / close dictionary |
| R | Restart stream |
| L | Toggle loop |
| O | Open file dialog |
| S | Open subtitle file dialog |
| C | Cycle content fit |
| , (comma) / . (period) | Frame step backward / forward |
| Home / End | Jump to previous / next subtitle cue |
| PageUp / PageDown | Previous / next playlist item |

## Usage

### Run the application

```bash
cargo run --release
```

Open a file with the `O` key, drag-and-drop, or pass a path (and optional subtitle) on the command line:

```bash
cargo run --release -- "C:/path/to/video.mp4" "C:/path/to/subtitle.srt"
```

### Use the library

The `iced_video_player` library exposes a `Video` media handle and a `VideoPlayer` Iced widget:

```rust
use iced_video_player::{Video, VideoPlayer};

fn main() -> iced::Result {
    iced::run(App::update, App::view)
}

struct App {
    video: Video,
}

impl Default for App {
    fn default() -> Self {
        App {
            video: Video::new(&url::Url::parse("file:///C:/my_video.mp4").unwrap()).unwrap(),
        }
    }
}

impl App {
    fn update(&mut self, _message: Message) {}
    fn view(&self) -> iced::Element<Message> {
        VideoPlayer::new(&self.video).into()
    }
}
```

`Video` exposes programmatic control — `set_paused`, `seek`, `set_speed`, `set_volume`, `set_muted`, `set_looping`, `restart_stream`, `step_one_frame`, `set_subtitle_url`, `set_subtitle_track`, `thumbnails`, plus accessors for position, duration, size, and framerate. Multiple `VideoPlayer` instances render concurrently in the same application (per-video wgpu resources, up to 256 instances).

## Building

GStreamer must be installed as a system dependency. Follow the [gstreamer-rs installation instructions](https://github.com/sdroege/gstreamer-rs#installation).

- **Linux**: use the system package manager, or enter the Nix development shell with `nix develop` (the flake targets `x86_64-linux` and bundles the needed GStreamer plugins and Vulkan).
- **Windows (MSVC/MinGW)** and **macOS**: follow the gstreamer-rs guide.
- Embedded PGS subtitle OCR and the Youdao dictionary WebView require **Windows 10+**; on other platforms these features are compiled out (platform-specific code is gated by `#[cfg]`).
- `ffmpeg` on `PATH` is required for embedded subtitle extraction.

```bash
cargo build            # debug build
cargo build --release  # optimized build
cargo check            # type-check without producing binaries
```

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                    ELP11 Application                   │
│  ┌──────────┐  ┌───────────┐  ┌────────────────────┐  │
│  │  Video   │  │VideoPlayer │  │  Sidebar          │  │
│  │ (handle) │  │ (widget)   │  │  Dict / Subs /    │  │
│  │          │  │            │  │  Playlist / Set.  │  │
│  └────┬─────┘  └─────┬──────┘  └────────────────────┘  │
└───────┼──────────────┼────────────────────────────────┘
        │              │  VideoPrimitive (custom wgpu primitive)
        │              ▼
        │    ┌──────────────────┐
        │    │   VideoPipeline   │  per-video wgpu resources
        │    │  (iced_wgpu)     │  (Y as R8Unorm, UV as Rg8Unorm)
        │    │ ┌──────────────┐ │
        │    │ │ WGSL Shader  │ │  BT.709 YUV→RGB on GPU
        │    │ └──────────────┘ │
        │    └──────────────────┘
        ▼
   ┌─────────────────────────┐
   │   GStreamer Pipeline     │  playbin
   │  ├── video → appsink    │──► Worker Thread ──► Frame Buffer
   │  └── text  → appsink    │──► Subtitle Buffer
   └─────────────────────────┘
```

The `Video` handle constructs a `playbin` pipeline that forces NV12 output into an `appsink`. A background worker thread pulls samples for the lifetime of the `Video` and writes them into a shared frame buffer; an atomic flag signals the render path that new data is available. The `VideoPlayer` widget submits a `VideoPrimitive` each frame; the `VideoPipeline` (implementing `iced_wgpu`'s primitive pipeline trait) lazily creates per-video textures, uploads Y/UV planes respecting the GStreamer `VideoMeta` stride, and renders a textured quad with the YUV→RGB shader.

A pure-Rust PGS decoder (`pgs` module) handles RLE decompression, palette parsing, and YUV→RGBA conversion for Blu-ray bitmap subtitles, supporting both 13-byte `.sup` segment headers and 3-byte raw segment headers.

## License

Licensed under either

- [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT](http://opensource.org/licenses/MIT)

at your option.
