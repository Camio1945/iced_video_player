<!--
Sync Impact Report
==================
Version change: 0.0.0 → 1.0.0 (initial ratification)
Modified principles: N/A (initial creation)
Added sections:
  - Core Principles (5 principles)
  - Technical Constraints
  - Development Workflow
  - Governance
Removed sections: None
Templates requiring updates:
  - .specify/templates/plan-template.md ✅ (Constitution Check section aligns with principles)
  - .specify/templates/spec-template.md ✅ (Requirements section aligns with principles)
  - .specify/templates/tasks-template.md ✅ (Task phases align with principles)
  - .specify/templates/checklist-template.md ✅ (No changes required)
  - .github/agents/speckit.constitution.agent.md ✅ (No outdated references)
Follow-up TODOs: None
-->

# iced_video_player Constitution

## Core Principles

### I. GPU-First Rendering
Video frame rendering MUST go through the custom wgpu pipeline. The YUV-to-RGB
color space conversion MUST execute on the GPU via the WGSL fragment shader
implementing the BT.709 standard. CPU-side YUV-to-RGBA conversion is permitted
only as a fallback (e.g., for thumbnail generation). Raw frame data from
GStreamer (NV12 format) MUST be uploaded to GPU textures respecting the
VideoMeta stride for correct row pitch.

### II. GStreamer Backend (NON-NEGOTIABLE)
All media decoding, demuxing, and pipeline management MUST rely on GStreamer's
`playbin` element. No alternative media backends (ffmpeg, mpv, etc.) are
permitted without an explicit architectural decision recorded and approved. The
pipeline configuration (video-sink, text-sink, caps negotiation) MUST be
declared explicitly in `Video::new()`, not abstracted behind runtime plugin
selection.

### III. Widget Composability
The `VideoPlayer` widget MUST implement Iced's `Widget` trait and follow its
layout/draw/update contract exactly. The public API MUST expose programmatic
control (pause, seek, volume, speed, looping) without bundling any GUI controls
(buttons, sliders, labels). Consumers are responsible for building their own
control surfaces using the `Video` handle's methods and callbacks. No user-facing
UI elements SHALL be included in the crate's public widget.

### IV. Error Resilience
All fallible operations MUST return the unified `Error` enum from `src/lib.rs`.
GStreamer bus errors MUST be drained and surfaced through the `on_error` callback.
Pipeline state changes that fail MUST propagate a `StateChangeError`. The worker
thread responsible for pulling frames from `appsink` MUST handle disconnection
and timeout gracefully without panicking. Lock poisoning on the shared `Frame`
mutex MUST be caught and converted to a recoverable error.

### V. Cross-Platform Compatibility
The crate MUST compile and function on Linux, Windows (MSVC and MinGW), and
macOS. Platform-specific code (e.g., Windows OCR for PGS subtitle conversion)
MUST be gated behind `#[cfg(...)]` attributes. The Nix flake targets `x86_64-linux`
only but MUST NOT prevent non-Nix builds on other platforms. GStreamer runtime
dependencies MUST be documented per-platform following the gstreamer-rs
installation guide.

## Technical Constraints

- **Language**: Rust edition 2024 with resolver 2.
- **GUI Framework**: Iced 0.14 with `image`, `advanced`, `wgpu`, and `svg` features.
- **GStreamer bindings**: gstreamer 0.23, gstreamer-app 0.23, gstreamer-video 0.23,
  gstreamer-base 0.23; glib 0.20 for GObject type system.
- **GPU**: wgpu backend (bridged through `iced_wgpu` 0.14); WGSL shader for YUV→RGB.
- **Error handling**: `thiserror` for the unified `Error` enum.
- **Logging**: `log` crate facade (no hard dependency on a specific logger impl).
- **Media URIs**: Parsed via the `url` crate (version 2).
- **Windows-only dependency**: `windows` 0.61 for PGS bitmap subtitle OCR — MUST
  remain behind `#[cfg(target_os = "windows")]`.

## Development Workflow

- **Build**: `cargo build` and `cargo build --release`. Always run `cargo check`
  before committing to verify compilation.
- **Testing**: The `minimal` example in `examples/minimal.rs` serves as the primary
  integration test. It MUST remain functional and demonstrate pause, loop, and seek.
  A test video at `.media/test.mp4` is required locally (excluded from the crate
  package by `Cargo.toml`).
- **Nix**: The flake provides a hermetic build environment on Linux. Non-Nix users
  MUST be able to build after installing GStreamer manually.
- **Code quality**: All public API items MUST have doc comments. Internal modules
  (`Internal`, frame worker, GPU pipeline) SHOULD have explanatory comments for
  non-obvious operations (AV sync latency tracking, texture stride handling).

## Governance

This constitution supersedes all other development practices for this project.
Amendments require:
1. A documented proposal explaining the change and its rationale.
2. Review against all dependent templates (plan, spec, tasks, checklist).
3. A MAJOR or MINOR version bump per semantic versioning rules:
   - MAJOR: Removal or redefinition of a core principle.
   - MINOR: New principle or section added.
   - PATCH: Clarifications, wording, typo fixes.

All feature specifications (generated via `/speckit.specify`) and implementation
plans (generated via `/speckit.plan`) MUST include a Constitution Check section
verifying compliance with these principles. Any violation MUST be explicitly
justified in the plan's Complexity Tracking table.

For runtime development guidance, refer to `CODEBUDDY.md` in the repository root.

**Version**: 1.0.0 | **Ratified**: 2026-07-23 | **Last Amended**: 2026-07-23
