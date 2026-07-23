//! Extract an embedded English subtitle stream from a video file into an
//! external `.srt` file next to the video.
//!
//! - Text-based streams (SRT/ASS/VTT) are converted directly with ffmpeg.
//! - PGS bitmap streams are extracted with ffmpeg, decoded to images with
//!   the built-in PGS decoder, and converted to text with Windows.Media.Ocr.

use std::path::{Path, PathBuf};

/// Extract subtitle stream `sub_index` (ffmpeg `0:s:N`) from `video_path`
/// into `<video_stem>.en.srt` next to the video.  Returns the SRT path.
/// If the SRT already exists (e.g. from a previous run), it is reused.
pub fn extract_embedded_subtitle(
    video_path: &str,
    sub_index: i32,
    is_pgs: bool,
) -> Result<PathBuf, String> {
    let video = Path::new(video_path);
    let dir = video.parent().ok_or("video has no parent directory")?;
    let stem = video
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid video file name")?;
    let out_srt = dir.join(format!("{stem}.en.srt"));
    if out_srt.exists() {
        return Ok(out_srt);
    }

    let ffmpeg = find_ffmpeg()?;
    if is_pgs {
        extract_pgs_to_srt(&ffmpeg, video_path, sub_index, &out_srt)?;
    } else {
        let map = format!("0:s:{sub_index}");
        run_ffmpeg(
            &ffmpeg,
            &["-y", "-i", video_path, "-map", &map, "-c:s", "srt"],
            &out_srt,
        )?;
    }
    if !out_srt.exists() {
        return Err("ffmpeg produced no output file".to_string());
    }
    Ok(out_srt)
}

fn extract_pgs_to_srt(
    ffmpeg: &Path,
    video_path: &str,
    sub_index: i32,
    out_srt: &Path,
) -> Result<(), String> {
    let sup = std::env::temp_dir().join(format!(
        "ivp_extract_{}_{}.sup",
        std::process::id(),
        sub_index
    ));
    let map = format!("0:s:{sub_index}");
    run_ffmpeg(
        ffmpeg,
        &["-y", "-i", video_path, "-map", &map, "-c", "copy"],
        &sup,
    )?;

    let data = std::fs::read(&sup).map_err(|e| format!("read {}: {e}", sup.display()))?;
    let _ = std::fs::remove_file(&sup);
    let sets = iced_video_player::pgs::parse_sup(&data);
    if sets.is_empty() {
        return Err("no PGS display sets found in stream".to_string());
    }

    let entries = ocr_display_sets(&sets)?;
    if entries.is_empty() {
        return Err("OCR produced no subtitle text".to_string());
    }
    write_srt(&entries, out_srt)
}

/// OCR every content display set and merge consecutive identical lines.
/// Each entry is (start_seconds, end_seconds, text).
fn ocr_display_sets(
    sets: &[iced_video_player::pgs::PgsDisplaySet],
) -> Result<Vec<(f64, f64, String)>, String> {
    let ocr = win_ocr::WinOcr::new()?;
    let mut entries: Vec<(f64, f64, String)> = Vec::new();

    for (i, set) in sets.iter().enumerate() {
        let Some(img) = &set.image else { continue };
        let Some(text) = ocr.recognize(&img.rgba, img.width, img.height) else {
            continue;
        };
        let next_pts = sets.get(i + 1).map(|n| n.pts_seconds);
        let end = next_pts.unwrap_or(set.pts_seconds + 4.0);
        let end = end.clamp(set.pts_seconds + 0.5, set.pts_seconds + 10.0);

        if let Some(last) = entries.last_mut()
            && last.2 == text
            && set.pts_seconds - last.1 < 0.5
        {
            last.1 = end;
            continue;
        }
        entries.push((set.pts_seconds, end, text));
    }
    Ok(entries)
}

fn write_srt(entries: &[(f64, f64, String)], path: &Path) -> Result<(), String> {
    let mut s = String::new();
    for (i, (start, end, text)) in entries.iter().enumerate() {
        s.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_timestamp(*start),
            srt_timestamp(*end),
            text
        ));
    }
    std::fs::write(path, s).map_err(|e| format!("write {}: {e}", path.display()))
}

fn srt_timestamp(t: f64) -> String {
    let ms = (t * 1000.0).round().max(0.0) as u64;
    let (h, rem) = (ms / 3_600_000, ms % 3_600_000);
    let (m, rem) = (rem / 60_000, rem % 60_000);
    let (sec, msec) = (rem / 1000, rem % 1000);
    format!("{h:02}:{m:02}:{sec:02},{msec:03}")
}

fn find_ffmpeg() -> Result<PathBuf, String> {
    for candidate in ["ffmpeg", "ffmpeg.exe"] {
        if std::process::Command::new(candidate)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
        {
            return Ok(PathBuf::from(candidate));
        }
    }
    Err("ffmpeg not found on PATH (required for subtitle extraction)".to_string())
}

fn run_ffmpeg(ffmpeg: &Path, args: &[&str], output: &Path) -> Result<(), String> {
    let out = std::process::Command::new(ffmpeg)
        .args(args)
        .arg(output)
        .output()
        .map_err(|e| format!("run ffmpeg: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("unknown error")
        ));
    }
    Ok(())
}

/// Windows.Media.Ocr wrapper (built into Windows 10+, no external tools).
mod win_ocr {
    use windows::Globalization::Language;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
    use windows::core::HSTRING;

    pub struct WinOcr {
        engine: OcrEngine,
    }

    impl WinOcr {
        pub fn new() -> Result<Self, String> {
            let lang = Language::CreateLanguage(&HSTRING::from("en-US"))
                .map_err(|e| format!("OCR language: {e}"))?;
            let engine =
                OcrEngine::TryCreateFromLanguage(&lang).map_err(|e| format!("OCR engine: {e}"))?;
            Ok(Self { engine })
        }

        /// OCR a PGS subtitle bitmap.  Returns `None` when no text is found.
        pub fn recognize(&self, rgba: &[u8], w: u32, h: u32) -> Option<String> {
            let bitmap = self.build_bitmap(rgba, w, h)?;
            let result = self.engine.RecognizeAsync(&bitmap).ok()?.get().ok()?;
            let text = result.Text().ok()?.to_string_lossy();
            let text = text.trim().to_string();
            (!text.is_empty()).then_some(text)
        }

        /// Preprocess a raw PGS bitmap for OCR: extract the bright glyph fill
        /// (luminance-based, so the dark outline is dropped and adjacent
        /// glyphs don't merge), upscale 3x, then build a SoftwareBitmap by
        /// decoding an in-memory BMP stream.  (Constructing the bitmap via
        /// `SoftwareBitmap::Create` + `LockBuffer` yields images the OCR
        /// engine silently rejects; the decoder path works reliably.)
        fn build_bitmap(
            &self,
            rgba: &[u8],
            w: u32,
            h: u32,
        ) -> Option<windows::Graphics::Imaging::SoftwareBitmap> {
            let bmp_data = Self::create_gray_bmp(rgba, w, h)?;
            let stream = InMemoryRandomAccessStream::new().ok()?;
            let writer = DataWriter::CreateDataWriter(&stream).ok()?;
            writer.WriteBytes(&bmp_data).ok()?;
            writer.StoreAsync().ok()?.get().ok()?;
            writer.DetachStream().ok()?;
            stream.Seek(0).ok()?;

            let decoder = windows::Graphics::Imaging::BitmapDecoder::CreateAsync(&stream)
                .ok()?
                .get()
                .ok()?;
            decoder.GetSoftwareBitmapAsync().ok()?.get().ok()
        }

        /// Convert RGBA pixels to a grayscale (ink=black, bg=white) BMP,
        /// upscaled 3x for better OCR accuracy.
        fn create_gray_bmp(rgba: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
            let gray = image::ImageBuffer::from_fn(w, h, |x, y| {
                let i = ((y * w + x) * 4) as usize;
                let (r, g, b, a) = (
                    rgba[i] as u32,
                    rgba[i + 1] as u32,
                    rgba[i + 2] as u32,
                    rgba[i + 3],
                );
                let lum = (299 * r + 587 * g + 114 * b) / 1000;
                let ink = a > 127 && lum > 128;
                let v = if ink { 0u8 } else { 255u8 };
                image::Rgba([v, v, v, 255])
            });
            let resized = image::imageops::resize(
                &gray,
                w * 3,
                h * 3,
                image::imageops::FilterType::CatmullRom,
            );
            let mut bmp = std::io::Cursor::new(Vec::new());
            resized.write_to(&mut bmp, image::ImageFormat::Bmp).ok()?;
            Some(bmp.into_inner())
        }
    }
}
