use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DictEntry {
    pub word: String,
    pub meanings: Vec<DictMeaning>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DictMeaning {
    #[serde(rename = "partOfSpeech")]
    pub part_of_speech: String,
    pub definitions: Vec<DictDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DictDefinition {
    pub definition: String,
    pub example: Option<String>,
}

/// Look up a word in the Free Dictionary API and return a formatted
/// multi-line definition string.  Returns a user-friendly error
/// message on network / parse failure.
pub fn lookup(name: &str) -> String {
    let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", name);
    match ureq::get(&url).call() {
        Ok(resp) => match resp.into_string() {
            Ok(body) => match serde_json::from_str::<Vec<DictEntry>>(&body) {
                Ok(entries) if !entries.is_empty() => format_entry(&entries[0]),
                _ => format!("No definition found for \"{}\"", name),
            },
            Err(_) => format!("Could not read response for \"{}\"", name),
        },
        Err(_) => format!("Could not look up \"{}\" (network error)", name),
    }
}

fn format_entry(e: &DictEntry) -> String {
    let mut r = format!("Word: {}\n\n", e.word);
    for m in &e.meanings {
        r.push_str(&format!("[{}]\n", m.part_of_speech));
        for (i, d) in m.definitions.iter().enumerate() {
            if i >= 3 {
                break;
            }
            r.push_str(&format!("  {}. {}\n", i + 1, d.definition));
            if let Some(ex) = &d.example {
                r.push_str(&format!("     e.g. \"{}\"\n", ex));
            }
        }
        r.push('\n');
    }
    r
}
