pub fn format_time(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

pub fn clean_subtitle_text(raw: &str) -> String {
    let mut s = raw.to_string();
    s = s
        .replace("<b>", "")
        .replace("</b>", "")
        .replace("<i>", "")
        .replace("</i>", "")
        .replace("<u>", "")
        .replace("</u>", "")
        .replace("<font", "X")
        .replace("</font>", "")
        .replace("\\N", "\n")
        .replace("\\n", "\n")
        .replace("\\h", " ");
    s = s
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&lt;", "<")
        .replace("&#60;", "<")
        .replace("&gt;", ">")
        .replace("&#62;", ">")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ");
    s.trim().to_string()
}

/// Stop words kept for potential future use (e.g. excluding common words
/// from vocabulary extraction).  All subtitle words are currently
/// clickable, so this list is no longer consulted at render time.
#[allow(dead_code)]
pub const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "if", "then", "else", "when", "at", "by", "for", "from",
    "in", "of", "on", "to", "with", "as", "is", "was", "are", "were", "be", "been", "being", "am",
    "do", "does", "did", "done", "have", "has", "had", "having", "will", "would", "should",
    "could", "can", "may", "might", "must", "shall", "it", "its", "this", "that", "these", "those",
    "i", "me", "my", "we", "us", "our", "you", "your", "he", "him", "his", "she", "her", "they",
    "them", "their", "what", "which", "who", "whom", "whose", "not", "no", "nor", "so", "too",
    "very", "just", "up", "down", "out", "about", "into", "over", "after", "before", "between",
    "through", "during", "above", "below", "re", "ve", "ll", "s", "t", "don", "didn", "doesn",
    "won", "isn", "aren", "couldn", "shouldn", "wouldn", "wasn", "weren", "hasn", "haven", "hadn",
    "mustn", "mightn", "apos", "ndash", "quot", "amp", "lt", "gt",
];
