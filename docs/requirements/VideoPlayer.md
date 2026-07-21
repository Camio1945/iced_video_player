# Video Player Application — Requirements Specification

> **Purpose of this document**: This is a complete, self-contained functional and technical
> specification for a desktop video player application built with **Rust + Iced (GUI) + GStreamer
> (media engine)**. It is written so that an AI coding tool can re-implement the application from
> scratch without seeing the original source code. Every behavior, layout rule, key binding,
> state transition, and edge case described here is derived from a working reference
> implementation.

---

## 1. Product Overview

A full-featured desktop video player with an emphasis on **language learning**: it plays local
video files via GStreamer, renders subtitles over the video, and lets the user **click any word
in the current subtitle line to look it up** in an online dictionary (Chinese translation +
English definitions) shown in a side panel.

### 1.1 Key capabilities (elevator pitch)

- Open and play local video files (mp4, mkv, avi, mov, webm, wmv, flv, m4v, mpg, mpeg, ogv).
- Full playback control: play/pause, seek bar with scrubbing, ±5s/±10s skip, frame stepping,
  variable speed (0.25×–4×), volume (0–200%) with mute, loop toggle.
- Multiple content-fit modes (Contain / Cover / Fill / None / ScaleDown) cycled at runtime.
- Fullscreen toggle.
- External subtitle loading (srt, ass, ssa, vtt, sub, smi) + **automatic subtitle discovery**
  next to the video file + detection of built-in text subtitle streams.
- Subtitles rendered as an overlay at the bottom of the video with **individually clickable
  words**.
- Clicking a subtitle word opens a **dictionary sidebar** (fixed 360px) showing Chinese
  translation (MyMemory API), phonetic/IPA, English definitions grouped by part-of-speech, and
  example sentences (dictionaryapi.dev).
- CLI arguments: `video-player <video-file> [subtitle-file]`.
- Dark theme, custom window icon, window title shows the current file name.

### 1.2 Target platform

- Windows (primary; `#![windows_subsystem = "windows"]`, `.ico` embedded via `winres` in
  `build.rs`), but the code must remain cross-platform (Linux/macOS) as long as GStreamer is
  installed.
- Rust edition 2024, resolver 2.

---

## 2. Technology Stack & Dependencies

| Crate          | Version | Purpose                                                        |
| -------------- | ------- | -------------------------------------------------------------- |
| `iced`         | 0.14    | GUI framework. Features: `image`, `advanced`, `wgpu`           |
| `iced_wgpu`    | 0.14    | wgpu backend (required for the custom video primitive)         |
| `gstreamer`    | 0.23    | Media pipeline (playbin)                                       |
| `gstreamer-app`| 0.23    | `appsink` for pulling decoded frames                           |
| `gstreamer-base`| 0.23   | base types                                                     |
| `gstreamer-video`| 0.23  | `VideoMeta` (stride/offset info for NV12 upload)               |
| `glib`         | 0.20    | GObject traits, `glib::Error`                                  |
| `log`          | 0.4     | logging facade                                                 |
| `thiserror`    | 1       | unified `Error` enum                                           |
| `url`          | 2       | media/subtitle URI handling                                    |
| `rfd`          | 0.15    | native file dialogs — **must use `AsyncFileDialog`** (see §6.4)|
| `ureq`         | 2       | blocking HTTP client for dictionary APIs (used inside async tasks) |
| `serde`/`serde_json` | 1 | JSON deserialization for dictionary APIs                   |
| `image`        | 0.25    | decode embedded PNG window icon                                |
| `winres`       | 0.1     | (build-dependency) embed `.ico` on Windows                     |

System requirement: **GStreamer runtime** must be installed on the host.

---

## 3. Architecture (two layers)

The implementation is split into a **reusable library** and an **application binary**:

```
crate `iced_video_player` (library)          binary `video-player` (src/main.rs)
┌──────────────────────────────────┐         ┌────────────────────────────────────┐
│ Video       (media handle)       │◀────────│ App / Message / VideoState         │
│ VideoPlayer (iced Widget)        │         │ update / view / subscription       │
│ VideoPipeline (wgpu primitive)   │         │ handlers, widgets, styles,         │
│ shader.wgsl (NV12→RGB, BT.709)   │         │ subtitle overlay, dictionary panel │
└──────────────────────────────────┘         └────────────────────────────────────┘
```

### 3.1 Library: media engine (`Video`)

- `Video::new(uri: &url::Url) -> Result<Video, Error>`:
  - Calls `gst::init()`.
  - Builds a `playbin` pipeline string:
    ```
    playbin uri="<uri>"
            text-sink="appsink name=iced_text sync=true drop=true"
            video-sink="videoscale ! videoconvert ! appsink name=iced_video drop=true
                        caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1"
    ```
  - Sets pipeline to `Playing`, then reads negotiated caps to extract **width, height,
    framerate, duration**.
  - Spawns a **background worker thread** (lives as long as the `Video`) that pulls video
    samples from `iced_video` appsink (`try_pull_sample` / `try_pull_preroll`, 16 ms timeout)
    into a shared `Arc<Mutex<Frame>>`; an `AtomicBool upload_frame` flags new data. A second
    optional appsink `iced_text` provides subtitle text buffers.
  - Detects whether the source has a **built-in text-based subtitle stream**
    (`builtin_text_subtitle`; bitmap formats like PGS/DVD excluded).
- Internal state lives behind `RwLock<Internal>`; the `Video` handle is cloneable/shareable.
- AV-sync: the render path measures the latency between frame arrival (worker timestamp) and
  consumption (`draw`), keeps a running average, and periodically applies it to the pipeline's
  `av-offset` property.

**Public API of `Video` (all signatures normative):**

| Method | Behavior |
| --- | --- |
| `size() -> (i32, i32)` | Native resolution `(width, height)`. |
| `framerate() -> f64` | Frames per second. |
| `set_volume(f64)` / `volume() -> f64` | Linear volume multiplier; `1.0` = 100%. App allows 0.0–2.0. **Note:** after `set_volume`, re-apply `set_muted(muted())` because GStreamer un-mutes on volume change. |
| `set_muted(bool)` / `muted() -> bool` | Mute without touching volume. |
| `eos() -> bool` | End-of-stream flag. |
| `set_looping(bool)` / `looping() -> bool` | Loop flag (consumed by the widget on EOS). |
| `set_paused(bool)` / `paused() -> bool` | Pause state (drives pipeline state). |
| `seek(position: impl Into<Position>, accurate: bool) -> Result<(), Error>` | Seek. `accurate=true` → precise but slower (used for frame-step back); `false` for scrubbing/skips. |
| `step_one_frame()` | Sends `gst::event::Step` (1 buffer) — frame-step forward while paused. |
| `set_speed(f64) -> Result<(), Error>` / `speed() -> f64` | Playback rate, default 1.0. |
| `position() -> Duration` | Queried live from the pipeline (`query_position`). |
| `duration() -> Duration` | Cached media duration. |
| `restart_stream() -> Result<(), Error>` | Seek to 0, unpause, clear EOS flag. |
| `set_subtitle_url(&url::Url) -> Result<(), Error>` | Load external subtitle **without interrupting playback**: re-enable playbin text flag (bit 2 / value 4) if cleared, save position, go to `Ready`, set `suburi`, go to `Paused` (preroll, wait ≤5 s), seek back to saved position, restore `Playing` if it was playing. |
| `subtitle_url() -> Option<url::Url>` | Current `suburi`. |
| `subtitle_stream_count() -> i32` | `n-text` property. |
| `has_builtin_subtitles() -> bool` | True if a renderable **text** subtitle stream exists. |
| `pipeline() -> gst::Pipeline` | Clone of the underlying pipeline. |
| `thumbnails(...)` | Seek + capture + CPU-side YUV→RGBA conversion returning `image::Handle`s (exists in the library; the app does not use it). |

**Library `Error` enum** (thiserror): `Glib(glib::Error)`, `Bool(glib::BoolError)`, `Bus`,
`AppSink(String)`, `StateChange(StateChangeError)`, `Cast`, `Io(io::Error)`, `Uri`, `Caps`,
`Duration`, `Sync`, `Lock`, `Framerate(f64)`.

### 3.2 Library: widget (`VideoPlayer`)

`VideoPlayer<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>` implements iced's
`Widget` trait and borrows a `&'a Video`.

Builder methods: `new(&video)`, `.width(Length)`, `.height(Length)`,
`.content_fit(ContentFit)`, `.on_end_of_stream(Message)`, `.on_new_frame(Message)`,
`.on_subtitle_text(impl Fn(Option<String>) -> Message)`, `.on_error(impl Fn(&glib::Error) -> Message)`.

Widget behavior:

- `update()` is driven by the window's `RedrawRequested` event. Each redraw it drains the
  GStreamer bus:
  - `Error` → fire `on_error`.
  - `Eos` → if `looping`, `seek(0)` to restart; else pause and set `is_eos`, fire
    `on_end_of_stream`.
  - New uploaded frame → fire `on_new_frame`.
  - Subtitle text change → fire `on_subtitle_text` (`None` when cleared).
  - Always requests the next redraw: immediately while playing, or `request_redraw_at` with a
    **32 ms delay** when paused/EOS (avoids busy-waiting).
- `layout()` mirrors iced's `Image` layout: resolves native resolution against `width`/`height`
  `Length`s and `ContentFit` (Contain, Cover, Fill, None, ScaleDown).
- `draw()`: computes fitted bounds, swaps `upload_frame` to `false`, updates the AV-sync offset,
  and submits a `VideoPrimitive`; clips with `renderer.with_layer()` when content exceeds bounds.

### 3.3 Library: GPU path (`VideoPipeline`, `VideoPrimitive`, `shader.wgsl`)

- Custom wgpu pipeline keyed by a monotonically increasing `video_id` (atomic counter) so
  multiple players can coexist; entries stored in a `BTreeMap<u64, VideoEntry>` and cleaned in
  `trim()` when dead.
- Textures: Y plane as `R8Unorm` (full res), interleaved UV plane as `Rg8Unorm` (half res);
  linear clamp-to-edge sampler; dynamic uniform buffer with 256 slots for quad rects.
- `upload()` respects the **stride from `VideoMeta::stride()[0]`** when calling
  `queue.write_texture()` (row pitch may exceed width due to hardware alignment). Lazy init on
  first upload per video id.
- WGSL vertex shader builds a full-screen quad from `uniforms.rect` (x1,y1,x2,y2) using only
  the vertex index (6 vertices, 2 triangles, no vertex buffer). Fragment shader performs
  **BT.709 limited-range YUV→RGB** (Y normalized from 16–235, UV from 16–240), clamps to
  `[0,1]`, alpha = 1.

---

## 4. Application State Model

### 4.1 `VideoState` enum

```rust
enum VideoState {
    NoVideo,          // initial / after failed load → placeholder screen
    Loading(String),  // path being opened → "Loading video..." screen
    Ready(Video),     // playable
}
```

### 4.2 `App` struct fields (all normative)

| Field | Type | Default | Purpose |
| --- | --- | --- | --- |
| `video` | `VideoState` | `NoVideo` | video lifecycle |
| `position` | `f64` (seconds) | `0.0` | UI-side position cache (seek bar) |
| `dragging` | `bool` | `false` | true while the seek slider is held |
| `volume` | `f64` | `1.0` | 0.0–2.0 |
| `muted` | `bool` | `false` | |
| `looping` | `bool` | `false` | mirrors video looping for the toggle button |
| `speed` | `f64` | `1.0` | playback rate |
| `fullscreen` | `bool` | `false` | |
| `content_fit` | `iced::ContentFit` | `Contain` | cycles through 5 modes |
| `subtitle_text` | `String` | empty | current cleaned subtitle line(s) |
| `dict_word` | `String` | empty | word being looked up (empty ⇒ placeholder view) |
| `dict_phonetic` | `String` | empty | IPA string, rendered as `/…/` |
| `dict_chinese` | `String` | empty | Chinese translation |
| `dict_sections` | `Vec<DictSection>` | empty | definitions grouped by part of speech |
| `dict_examples` | `Vec<String>` | empty | up to 5 deduplicated examples |
| `dict_loading` | `bool` | `false` | show "Looking up..." placeholder |
| `dict_error` | `Option<String>` | `None` | "No definition found for …" |
| `current_file_path` | `Option<String>` | `None` | for window title |
| `window_id` | `Option<iced::window::Id>` | `None` | captured from `window::open_events` subscription, needed for fullscreen |
| `pending_subtitle` | `Option<PathBuf>` | `None` | CLI-provided subtitle, applied after open |

Helper methods on `App`: `with_video(f) -> Option<T>`, `with_video_mut(f) -> Option<R>` (only in
`Ready` state), `video_duration() -> f64`, `current_pos() -> f64` (live query; falls back to
cached `position`), `clear_dictionary()` (resets all `dict_*` fields).

### 4.3 `Message` enum (complete list)

`TogglePause`, `Seek(f64)`, `SeekRelease`, `SkipBack(i64)`, `SkipForward(i64)`,
`FrameStepForward`, `FrameStepBackward` (wired to keyboard only, no button), `EndOfStream`,
`NewFrame`, `PlaybackError(String)`, `OpenFile`, `FilePicked(Option<PathBuf>)`,
`FileOpened(Result<String, String>)`, `LoadSubtitle`, `SubtitlePicked(Option<PathBuf>)`,
`SubtitleText(String)`, `SearchWord(String)`, `DictionaryResult(DictResult)`,
`CloseDictionary`, `ToggleLoop`, `ToggleMute`, `SetVolume(f64)`, `SetSpeed(f64)`,
`ToggleFullscreen`, `CycleContentFit`, `KeyboardEvent(iced::keyboard::Event)`,
`WindowOpened(iced::window::Id)`.

---

## 5. Window & Application Setup

- `iced::application(boot, update, view)` with:
  - **Title**: `"Video Player"` when idle; `"<file-name> - Video Player"` when a file is loaded
    (file name extracted from `current_file_path`).
  - **Theme**: always `Theme::Dark`.
  - **Window**: initial size **1280×760**, min size **800×480**, start **maximized**, custom
    icon decoded from embedded `assets/icon.png` (PNG → RGBA → `window::icon::from_rgba`).
  - **Subscriptions**: `keyboard::listen()` → `KeyboardEvent`; `window::open_events()` →
    `WindowOpened` (stores the window id).
- `build.rs`: on Windows, embed `assets/icon.ico` via `winres`.
- Binary has `#![windows_subsystem = "windows"]` (no console window).

### 5.1 CLI arguments

`video-player [video-path] [subtitle-path]` (positional; both optional).

- With a video path: boot closure sets `VideoState::Loading(path)`, records
  `current_file_path`, stores `pending_subtitle` if given, converts the path to a `file://` URL
  (`Url::from_file_path`, falling back to `Url::parse("file:///<path>")`), and kicks off an
  async `Task` that probes `Video::new(&url)` → `FileOpened(Ok(path) | Err(msg))`.

---

## 6. UI Layout

Overall window root: a `Container` filling the window with background `rgb(0.1, 0.1, 0.12)`,
containing a `Column`:

```
Column (Fill × Fill)
├── Toolbar (Row)
└── Row (main area)
    ├── Player column (Fill) ── Column
    │   ├── Video area (Fill) [+ subtitle overlay in a Stack when subtitle_text ≠ ""]
    │   ├── Seek bar (Container padding [0,8])
    │   └── Controls row
    └── Dictionary sidebar (fixed width 360)
```

### 6.1 Toolbar (top)

Row, spacing 4, padding 4, vertically centered:

1. **"Open"** button (text size 12, padding [4,8], `ctrl_btn` style) → `OpenFile`.
2. **"Subtitle..."** button (same styling) → `LoadSubtitle`; **disabled when no video**.
3. Flexible space.
4. Time label `"<pos> / <dur>"` (size 12) using `format_time`:
   - `H:MM:SS` when ≥ 1 hour, else `M:SS` (minutes not zero-padded, seconds zero-padded;
     computed from truncated integer seconds).

### 6.2 Video area

- `VideoState::Ready` → `VideoPlayer` widget: `width(Fill)`, `height(Fill)`,
  `content_fit(app.content_fit)`, wired to `EndOfStream`, `NewFrame`,
  `SubtitleText(Option<String> → unwrap_or_default)`, `PlaybackError(glib::Error → String)`.
- `VideoState::Loading(path)` → centered column: "Loading video..." (size 18) + path
  (size 12), on `placeholder` background (`rgb(0.08, 0.08, 0.1)`).
- `VideoState::NoVideo` → centered column on `placeholder` background:
  - app icon (`assets/icon.png`, fixed width 140),
  - "No video loaded" (size 18),
  - "Click \"Open\" or press O to load a video" (size 14),
  - "Open Video File" button (padding [8,20]) → `OpenFile`.

### 6.3 Subtitle overlay

When `subtitle_text` is non-empty, the video container is wrapped in a `Stack`; the subtitle is
a bottom-aligned container (padding `[0, 48]`) holding the clickable-word view:

- Background: `rgba(0, 0, 0, 0.50)`; inner padding `[8, 16]`; centered horizontally; width Fill.
- Font size 20, white text; line spacing 2 between display rows.
- Text pipeline (normative):
  1. Split on `\n`, trim, drop empty lines.
  2. **Greedy line merging**: consecutive source lines are joined with a space as long as the
     merged line stays ≤ **80 chars**.
  3. **Tokenize** into `Word` (alphabetic chars plus `'` and `-`) and `Punct` (everything else,
     one char each).
  4. **Wrap** tokens into visual lines ≤ 80 chars; a single token longer than 80 chars still
     gets its own line (never dropped).
  5. Each word token: split off trailing `'`/`-` into a non-clickable suffix; the core word is
     wrapped in a `MouseArea` with pointer cursor; pressing fires
     `SearchWord(word.to_lowercase())`. Punctuation renders as plain text with zero spacing
     between tokens (spacing 0 rows).
- **Every word is clickable**, including short/common words and words with apostrophes.

### 6.4 Seek bar

`Slider` over `0.0..=duration.max(0.01)`, value = `position`, step **0.5**, width Fill,
`on_release = SeekRelease`, padded `[0, 8]`.

Behavior:
- While dragging (`Seek(msg)`): set `dragging = true`, update cached `position`, and **pause
  the video** (live scrub preview comes from the widget still redrawing; the actual seek is
  deferred).
- On release (`SeekRelease`): `dragging = false`, `seek(position, accurate=false)`, then
  **unpause**.
- On `NewFrame`: if not dragging, refresh cached `position` from the live pipeline position.

### 6.5 Controls row (bottom)

Row, spacing 6, padding `[4, 8]`, vertically centered. Left group (media buttons, all disabled
when no video except where noted):

| Button | Glyph (Unicode) | Size | Message | Notes |
| --- | --- | --- | --- | --- |
| Skip −10 s | `⏪` U+23EA | 14 | `SkipBack(10)` | |
| Skip −5 s | `⏴` U+23F4 | 14 | `SkipBack(5)` | |
| Play/Pause | `▶` U+25B6 (paused) / `⏸` U+23F8 (playing) | 18 | `TogglePause` | `main_btn` (blue) style |
| Skip +5 s | `⏵` U+23F5 | 14 | `SkipForward(5)` | |
| Skip +10 s | `⏩` U+23E9 | 14 | `SkipForward(10)` | |
| Frame step | `\|▶` | 12 | `FrameStepForward` | **enabled only when a video is loaded AND paused** |

Skip semantics: target = `clamp(position ± secs, 0, duration)`; cached `position` updated;
seek with `accurate = false`.

Right group (after a flexible space):

1. Label "Speed:" (size 11).
2. **Speed PickList** — options `[0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0]`,
   selected = current speed, arrow handle, fixed width 80, advanced text shaping →
   `SetSpeed(f64)`.
3. **Loop** button `🔁` U+1F501 (size 14) → `ToggleLoop`; uses `active_btn` (green) style when
   looping, else `ctrl_btn`. Always clickable (message is a no-op without video).
4. **Mute** button `🔇` U+1F507 when muted / `🔊` U+1F50A when not (size 14) → `ToggleMute`.
5. **Volume slider** `0.0..=2.0`, step 0.05, fixed width 90 → `SetVolume(f64)`.
6. **Content-fit** button showing `format!("{:?}", content_fit)` (size 10) → cycles
   `Contain → Cover → Fill → None → ScaleDown → Contain`.
7. **Fullscreen** button `⛶` U+26F6 (size 14) → `ToggleFullscreen`.

### 6.6 Dictionary sidebar (always visible, fixed width 360)

Container styled `sidebar` (background `rgb(0.13, 0.13, 0.17)`, 1px border
`rgb(0.25, 0.25, 0.32)`), full height, containing a Column:

**Header** (`sidebar_header` style, bg `rgb(0.16, 0.16, 0.22)`, border `rgb(0.3, 0.3, 0.4)`),
row padding `[8, 10]`:
- No word: "📖  Dictionary" (size 14).
- With word: "📖  \<word\>" (size 15, color `rgb(0.95, 0.9, 0.6)`), plus a close button
  `✕` U+2715 (size 13, padding [1,6], `ctrl_btn`) → `CloseDictionary`.

**Body** (scrollable, `sidebar_body` style):
- `dict_loading` → centered: `⏳` (size 22) + "Looking up..." (size 12), padding `[24, 12]`.
- `dict_word` empty → centered, padding `[24, 14]`: `👈` (size 28, color
  `rgb(0.7, 0.7, 0.75)`), "Click a word in the subtitle" (size 13, `rgb(0.85, 0.85, 0.9)`),
  "The Chinese meaning will appear here." (size 11, `rgb(0.6, 0.6, 0.65)`).
- Otherwise, Column spacing 10, padding `[12, 12]`:
  1. **Chinese section** (if non-empty) in a `dict_section_card` (bg
     `rgba(0.22, 0.22, 0.3, 0.6)`, 1px border `rgb(0.3, 0.3, 0.4)`, rounded 6, padding
     `[10, 10]`): label "中文" (size 10, `rgb(0.75, 0.75, 0.8)`) + translation (size 20,
     `rgb(1.0, 0.82, 0.45)`).
  2. **Phonetic** (if non-empty): `"/<ipa>/"` size 13, `rgb(0.75, 0.8, 0.9)`.
  3. **Definitions** per section: `"[part-of-speech]"` (size 11, `rgb(0.55, 0.85, 0.6)`), then
     numbered definitions `"N. <definition>"` (size 12, `rgb(0.88, 0.88, 0.92)`, word-wrap) and
     optional indented quoted example `    “<example>”` (size 11, `rgb(0.62, 0.62, 0.7)`,
     curly quotes U+201C/U+201D). Sections separated by spacing 10, entries by 3.
  4. **Examples** (if non-empty): "Examples:" (size 11, `rgb(0.75, 0.75, 0.8)`) then bullets
     `"• <example>"` (size 11, `rgb(0.78, 0.78, 0.85)`, word-wrap).
  5. **Error** (if set): message in `rgb(1.0, 0.55, 0.55)`, size 12.

### 6.7 Button styles (normative colors)

| Style | Normal bg | Hovered bg | Pressed bg | Text | Border |
| --- | --- | --- | --- | --- | --- |
| `ctrl_btn` | rgb(0.2,0.2,0.25) | rgb(0.35,0.35,0.4) | rgb(0.25,0.25,0.3) | rgb(0.85,0.85,0.85) | rounded 4 |
| `main_btn` | rgb(0.2,0.45,0.75) | rgb(0.3,0.55,0.85) | rgb(0.2,0.4,0.7) | white | rounded 4 |
| `active_btn` | rgb(0.2,0.6,0.35) | rgb(0.3,0.7,0.4) | rgb(0.2,0.55,0.3) | white | rounded 4 |

---

## 7. Interaction Handlers (normative behavior)

| Message | Behavior |
| --- | --- |
| `TogglePause` | flip `paused` on the video (no-op without video). |
| `Seek(secs)` | `dragging = true`; cache `position = secs`; pause video. |
| `SeekRelease` | `dragging = false`; `seek(position, accurate=false)`; unpause. |
| `SkipBack(secs)` | `pos = max(position - secs, 0)`; cache; seek inaccurate. |
| `SkipForward(secs)` | `pos = min(position + secs, duration)`; cache; seek inaccurate. |
| `FrameStepForward` | `video.step_one_frame()`. |
| `FrameStepBackward` | `pos = max(position - 1/framerate, 0)`; cache; `seek(pos, accurate=true)`; ensure paused. |
| `EndOfStream` | no-op at app level (widget already handled loop/pause). |
| `NewFrame` | if not dragging: `position = video.position()`. |
| `PlaybackError(err)` | log to stderr (`eprintln!("Playback error: {}", err)`). |
| `OpenFile` | spawn async task with `rfd::AsyncFileDialog` (see note below): filter "Video Files" `[mp4, mkv, avi, mov, webm, wmv, flv, m4v, mpg, mpeg, ogv]` + "All Files" `[*]`; → `FilePicked`. |
| `FilePicked(Some(path))` | set `Loading(path)`, `current_file_path = path`, clear `subtitle_text`, `clear_dictionary()`, `pending_subtitle = None`; async-probe `Video::new(url)` → `FileOpened`. `None` → nothing. |
| `FileOpened(Ok(path))` | construct `Video::new(url)` (second time, now on the UI side); on success read `has_builtin_subtitles()`, set `Ready(video)`, `position = 0`, run **subtitle auto-apply** (§7.1); on failure → `NoVideo` + stderr log. `Err` → `NoVideo` + log. |
| `LoadSubtitle` | async dialog: filter "Subtitle Files" `[srt, ass, ssa, vtt, sub, smi]` + "All Files"; → `SubtitlePicked`. |
| `SubtitlePicked(Some(path))` | `video.set_subtitle_url(file_url)`; errors logged. |
| `SubtitleText(text)` | `subtitle_text = clean_subtitle_text(text)` (§7.2). |
| `SearchWord(word)` | set `dict_word = word`, `dict_loading = true`, clear other dict fields; async task `dict::lookup(word)` → `DictionaryResult`. |
| `DictionaryResult(res)` | store all fields; `dict_loading = false`. |
| `CloseDictionary` | `clear_dictionary()`. |
| `ToggleLoop` | flip looping on the video, mirror into `app.looping`. |
| `ToggleMute` | flip `app.muted`, apply `set_muted`. |
| `SetVolume(v)` | store + `set_volume(v)`. |
| `SetSpeed(s)` | store + `set_speed(s)` (errors ignored). |
| `ToggleFullscreen` | flip flag; `window::set_mode(window_id, Fullscreen \| Windowed)`. |
| `CycleContentFit` | Contain→Cover→Fill→None→ScaleDown→Contain. |
| `WindowOpened(id)` | store `window_id`. |

> **File-dialog note (important, non-obvious):** the dialogs MUST be opened with
> `rfd::AsyncFileDialog` inside a `Task::perform`. Calling the synchronous
> `rfd::FileDialog::pick_file()` on the Iced UI thread blocks the event loop and starves Windows
> COM messages, which makes the shell file list load extremely slowly ("Working on it...").

### 7.1 Subtitle auto-apply (after a video opens)

Order of precedence:

1. **CLI-provided subtitle** (`pending_subtitle`) → `set_subtitle_url`, done.
2. Else, if the video **has no built-in text subtitles** → run **auto-discovery**
   (`find_english_subtitle_file(video_path)`):
   - Look in the video's directory for files whose name starts with the video's file stem and
     whose extension (lowercased) ∈ `{srt, ass, ssa, vtt, sub, smi}`.
   - Prefer the candidate whose remainder after the stem is **only the extension**
     (e.g. `movie.srt` beats `movie.zh-CN.srt`) — that's treated as the default/English one.
   - Fallback: first candidate found.
   - Load it via `set_subtitle_url`.
3. If the video has built-in text subtitles, do nothing (playbin already renders them to the
   text appsink).

All subtitle errors are logged to stderr, never shown in the UI.

### 7.2 Subtitle text cleaning (`clean_subtitle_text`)

Applied to every incoming subtitle buffer before display:

- Strip tags: `<b>`, `</b>`, `<i>`, `</i>`, `<u>`, `</u>`, `</font>`; replace `<font` with `X`
  (defeats font tags with attributes).
- Replace escapes: `\N` → newline, `\n` → newline, `\h` → space.
- Replace HTML entities (named and numeric): `&apos;`/`&#39;` → `'`, `&quot;`/`&#34;` → `"`,
  `&amp;`/`&#38;` → `&`, `&lt;`/`&#60;` → `<`, `&gt;`/`&#62;` → `>`, `&nbsp;`/`&#160;` → space.
- Trim; return owned `String`.

---

## 8. Keyboard Shortcuts

Driven by `keyboard::listen()`; only `KeyPressed` events are handled.

| Key | Action |
| --- | --- |
| `Space` | Toggle pause |
| `←` (ArrowLeft) | Skip back 5 s |
| `→` (ArrowRight) | Skip forward 5 s |
| `↑` (ArrowUp) | Volume +0.05 (clamped to 2.0) |
| `↓` (ArrowDown) | Volume −0.05 (clamped to 0.0) |
| `F` | Toggle fullscreen |
| `M` | Toggle mute |
| `L` | Toggle loop |
| `[` | Speed −0.25 (min 0.25) |
| `]` | Speed +0.25 (max 4.0) |
| `,` | Frame step backward (accurate seek back 1/fps, stays paused) |
| `.` | Frame step forward |
| `O` | Open file dialog |
| `S` | Load subtitle dialog |
| `C` | Cycle content fit |
| `Escape` | If fullscreen → exit fullscreen; else if dictionary has a word → close dictionary; else nothing |

Character keys are matched case-insensitively (`f|F`, `m|M`, `l|L`, `o|O`, `s|S`, `c|C`).

---

## 9. Dictionary Lookup (network)

`dict::lookup(word) -> DictResult` runs **inside an async task** (blocking HTTP is fine there).

### 9.1 Chinese translation — MyMemory

- GET `https://api.mymemory.translated.net/get?q=<urlencoded word>&langpair=en|zh-CN`
  (query built with `url::form_urlencoded`).
- Parse `responseData.translatedText` (serde rename `responseData`/`translatedText`).
- **Cleaning**: trim; if empty or contains `MYMEMORY WARNING` or `PLEASE SELECT TWO DISTINCT`
  → treat as empty (the API echoes uppercase warnings for OOV/empty queries).
- Any network/parse error → empty string (never propagate).

### 9.2 English definitions — dictionaryapi.dev

- GET `https://api.dictionaryapi.dev/api/v2/entries/en/<word>`.
- Parse `Vec<DictEntry>`; use the **first entry only**.
- Phonetic: `entry.phonetic`, else first non-empty `phonetics[].text`, else empty.
- For each meaning: `partOfSpeech` → section title; take **at most 3 definitions** each with
  its optional example.
- Collect up to **5 unique examples** across all definitions (deduplicated, first-seen order).
- Any error → empty everything.

### 9.3 Result assembly

`DictResult { word, chinese, phonetic, sections, examples, error }` where
`error = Some(format!("No definition found for \"{word}\""))` only when **both** Chinese and
English lookups came back empty.

Data shapes:

```rust
struct DictSection { part_of_speech: String, definitions: Vec<(String, Option<String>)> }
struct DictResult  { word: String, chinese: String, phonetic: String,
                     sections: Vec<DictSection>, examples: Vec<String>,
                     error: Option<String> }
```

---

## 10. Message Flows (sequence summaries)

### 10.1 Opening a video (menu/button/CLI)

```
OpenFile → AsyncFileDialog → FilePicked(Some(path))
  → state = Loading(path); clear subtitle/dict; pending_subtitle = None
  → task: Video::new(url) probe
  → FileOpened(Ok(path))
       → Video::new(url) (real handle)
       → read has_builtin_subtitles
       → state = Ready(video); position = 0
       → subtitle auto-apply (§7.1)
  → FileOpened(Err) → state = NoVideo; log
```

### 10.2 Per-frame UI update

```
VideoPlayer::update (on RedrawRequested)
  → bus drain: Error → PlaybackError; Eos → loop-restart or EndOfStream
  → new frame → NewFrame → position = video.position() (unless dragging)
  → subtitle change → SubtitleText → clean_subtitle_text → overlay re-render
```

### 10.3 Word lookup

```
click word in subtitle → SearchWord(lowercased)
  → dict_loading = true; task dict::lookup(word)
  → DictionaryResult → populate sidebar; dict_loading = false
```

---

## 11. Non-functional Requirements

1. **Responsiveness**: no blocking calls on the UI thread — file dialogs, video probing, and
   dictionary HTTP all run in `Task::perform`.
2. **Continuous redraw**: the widget self-schedules redraws (immediate while playing, 32 ms
   cadence while paused/EOS).
3. **GPU correctness**: NV12 upload must honor GStreamer `VideoMeta` stride; BT.709 limited
   range color conversion; multiple concurrent `VideoPlayer`s supported via per-id GPU entries.
4. **Resource lifecycle**: dropping a `Video` stops its worker thread and pipeline; GPU entries
   are trimmed by liveness flag.
5. **AV sync**: running-average latency compensation written to playbin's `av-offset`.
6. **Error philosophy**: playback/subtitle/dictionary errors are logged to stderr; the UI never
   shows modal error dialogs; failures degrade to the `NoVideo` placeholder or an empty
   dictionary result with an inline error line.
7. **Theme**: dark theme enforced regardless of OS preference.

---

## 12. Build & Run

```bash
cargo build            # library + binary
cargo run --bin video-player                       # starts with empty state
cargo run --bin video-player -- D:\videos\a.mp4    # open directly
cargo run --bin video-player -- D:\videos\a.mp4 D:\videos\a.srt
```

Assets required at compile time: `assets/icon.png` (window icon + no-video placeholder) and
`assets/icon.ico` (Windows executable icon via `build.rs`).

---

## 13. Edge Cases & Gotchas (must preserve)

1. **GStreamer un-mutes on volume change** — always re-apply mute after `set_volume`.
2. **Synchronous `rfd::FileDialog` on the UI thread stalls Windows shell enumeration** — use
   `rfd::AsyncFileDialog`.
3. **`set_subtitle_url` must preserve playback position and pause state** (Ready → set suburi →
   Paused/preroll → seek back → restore Playing) and must re-enable playbin text flag bit 2.
4. **Seek slider**: pause on drag-start, seek+unpause on release; position updates from
   `NewFrame` are suppressed while dragging.
5. **Frame-step backward** uses an *accurate* seek (`position - 1/framerate`, clamped at 0) and
   leaves the video paused; frame-step forward uses GStreamer's `Step` event and its button is
   only enabled while paused.
6. **Loop** is implemented by the widget: on EOS with `looping == true`, `seek(0)`; otherwise
   pause and flag EOS.
7. **Subtitle overlay** merges short source lines greedily (≤ 80 chars) before word wrapping;
   tokens longer than the limit still render on their own line.
8. **Volume range is 0.0–2.0** (200%), slider step 0.05; speed range 0.25–4.0 with the fixed
   picklist values listed in §6.5.
9. **Fullscreen requires the window id**, captured once via the `window::open_events`
   subscription.
10. **`duration.max(0.01)`** guards the seek slider range before metadata is available.
11. **MyMemory warning strings** must be filtered or they would render as fake translations.
12. The window title must follow `"<file-name> - Video Player"` once a file is loaded.
