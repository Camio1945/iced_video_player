/// Search the same directory as `video_path` for an external subtitle file
/// whose name starts with the video's stem.  When multiple candidates exist,
/// the one **without** a language suffix (e.g. `.zh-CN`) is preferred as the
/// default / English subtitle.
pub fn find_english_subtitle_file(video_path: &str) -> Option<std::path::PathBuf> {
    let video_path = std::path::Path::new(video_path);
    let dir = video_path.parent()?;
    let stem = video_path.file_stem()?.to_str()?;

    let mut candidates = collect_subtitle_candidates(dir, stem)?;

    if candidates.is_empty() {
        return None;
    }

    pick_best_subtitle_candidate(&mut candidates)
}

fn collect_subtitle_candidates(
    dir: &std::path::Path,
    stem: &str,
) -> Option<Vec<(std::path::PathBuf, String)>> {
    let subtitle_exts = ["srt", "ass", "ssa", "vtt", "sub", "smi"];
    let mut candidates: Vec<(std::path::PathBuf, String)> = Vec::new();

    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(remainder) = name.strip_prefix(stem) else {
            continue;
        };
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_str()?.to_lowercase();
            if subtitle_exts.contains(&ext_lower.as_str()) {
                candidates.push((path.clone(), remainder.to_string()));
            }
        }
    }
    Some(candidates)
}

/// Prefer the default/English subtitle: the one where the remainder
/// consists of only the extension (e.g. ".srt" → empty, vs ".zh-CN.srt").
fn pick_best_subtitle_candidate(
    candidates: &mut Vec<(std::path::PathBuf, String)>,
) -> Option<std::path::PathBuf> {
    for (path, remainder) in candidates.iter() {
        let remainder_no_ext = if let Some(dot_pos) = remainder.rfind('.') {
            &remainder[..dot_pos]
        } else {
            remainder
        };
        if remainder_no_ext.is_empty() {
            return Some(path.clone());
        }
    }
    // Fallback: return the first candidate
    Some(candidates.remove(0).0)
}
