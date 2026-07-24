//! Parse external subtitle files (SRT, VTT, ASS/SSA) into a flat list of
//! timed cues, used for keyboard navigation between subtitles (Home/End).

use std::path::Path;

/// A single subtitle cue with start/end times in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubtitleCue {
    pub start: f64,
    pub end: f64,
}

/// Parse a subtitle file into a list of cues sorted by start time.
///
/// Supports `.srt`, `.vtt`, `.ass` and `.ssa`. Returns an empty vector for
/// unrecognised formats or unreadable files (best-effort: subtitle display
/// via GStreamer is unaffected).
pub fn parse_subtitle_file(path: &Path) -> Vec<SubtitleCue> {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    // Normalise line endings so block splitting works on Windows (\r\n) and
    // classic Mac (\r) files alike — otherwise `split("\n\n")` treats the whole
    // file as a single block and only the first cue is parsed.
    let content = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut cues = match ext.to_lowercase().as_str() {
        "ass" | "ssa" => parse_ass(&content),
        // SRT and VTT share the same "block with a --> timing line" shape.
        _ => parse_block_timed(&content),
    };
    cues.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    cues
}

/// Parse SRT/VTT-style content: blocks separated by blank lines, each with a
/// `start --> end` timing line.
fn parse_block_timed(content: &str) -> Vec<SubtitleCue> {
    let mut cues = Vec::new();
    for block in content.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let Some(timing_line) = block.lines().find(|l| l.contains("-->")) else {
            continue;
        };
        let Some((start_str, end_str)) = timing_line.split_once("-->") else {
            continue;
        };
        // The end side may carry trailing VTT cue settings, e.g.
        // "00:00:03.000 align:start position:10%". Take the first token.
        let end_str = end_str.trim().split_whitespace().next().unwrap_or("");
        let start = parse_timestamp(start_str.trim());
        let end = parse_timestamp(end_str);
        if let (Some(start), Some(end)) = (start, end) {
            cues.push(SubtitleCue { start, end });
        }
    }
    cues
}

/// Parse ASS/SSA `[Events]` Dialogue lines using the `Format:` header to
/// locate the `Start`/`End` columns.
fn parse_ass(content: &str) -> Vec<SubtitleCue> {
    let mut cues = Vec::new();
    let mut in_events = false;
    let mut format: Option<Vec<String>> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_events = line.eq_ignore_ascii_case("[events]");
            continue;
        }
        if !in_events {
            continue;
        }
        if let Some(rest) = line.strip_prefix("Format:") {
            format = Some(rest.split(',').map(|s| s.trim().to_lowercase()).collect());
            continue;
        }
        let Some(rest) = line.strip_prefix("Dialogue:") else {
            continue;
        };
        if let Some(cue) = parse_ass_dialogue(rest, format.as_deref()) {
            cues.push(cue);
        }
    }
    cues
}

/// Parse a single ASS `Dialogue:` line (the part after `Dialogue:`) into a
/// cue, using the `[Events] Format:` header to locate the Start/End columns.
fn parse_ass_dialogue(rest: &str, format: Option<&[String]>) -> Option<SubtitleCue> {
    let fmt = format?;
    let start_idx = fmt.iter().position(|f| f == "start")?;
    let end_idx = fmt.iter().position(|f| f == "end")?;
    // Start/End always precede Text, so splitting on commas is safe here even
    // though the Text field itself may contain commas.
    let fields: Vec<&str> = rest.split(',').collect();
    let start = fields
        .get(start_idx)
        .and_then(|s| parse_timestamp(s.trim()))?;
    let end = fields
        .get(end_idx)
        .and_then(|s| parse_timestamp(s.trim()))?;
    Some(SubtitleCue { start, end })
}

/// Parse a timestamp in `HH:MM:SS,mmm`, `HH:MM:SS.mmm`, `H:MM:SS.cc` or
/// `MM:SS.mmm` form into seconds. Returns `None` for unparseable input.
fn parse_timestamp(s: &str) -> Option<f64> {
    let s = s.trim().replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    let (h, m, sec) = match parts.len() {
        3 => (parts[0], parts[1], parts[2]),
        2 => ("0", parts[0], parts[1]),
        _ => return None,
    };
    let h: f64 = h.parse().ok()?;
    let m: f64 = m.parse().ok()?;
    let sec: f64 = sec.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_srt() {
        let srt =
            "1\n00:00:01,000 --> 00:00:03,000\nHello\n\n2\n00:00:05,500 --> 00:00:07,000\nWorld\n";
        let cues = parse_block_timed(srt);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start, 1.0);
        assert_eq!(cues[0].end, 3.0);
        assert_eq!(cues[1].start, 5.5);
        assert_eq!(cues[1].end, 7.0);
    }

    #[test]
    fn parse_srt_crlf() {
        // Windows line endings: blocks separated by "\r\n\r\n". Without
        // normalisation, `split("\n\n")` treats the whole file as one block
        // and only the first cue is parsed.
        let srt = "1\r\n00:00:01,000 --> 00:00:03,000\r\nHello\r\n\r\n2\r\n00:00:05,500 --> 00:00:07,000\r\nWorld\r\n";
        let normalized = srt.replace("\r\n", "\n").replace('\r', "\n");
        let cues = parse_block_timed(&normalized);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start, 1.0);
        assert_eq!(cues[1].start, 5.5);
    }

    #[test]
    fn parse_vtt_with_settings() {
        let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:03.000 align:start\nHello\n\n00:00:05.000 --> 00:00:07.000\nWorld\n";
        let cues = parse_block_timed(vtt);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].end, 3.0);
        assert_eq!(cues[1].start, 5.0);
    }

    #[test]
    fn parse_ass_dialogue() {
        let ass = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, Effect, Text\nDialogue: 0,0:00:01.00,0:00:03.50,Default,,0,0,0,Hello\nDialogue: 0,0:00:05.00,0:00:07.00,Default,,0,0,0,World\n";
        let cues = parse_ass(ass);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start, 1.0);
        assert_eq!(cues[0].end, 3.5);
        assert_eq!(cues[1].start, 5.0);
    }
}
