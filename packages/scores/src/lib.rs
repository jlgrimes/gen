include!(concat!(env!("OUT_DIR"), "/scores.rs"));

/// A score with its name and content
#[derive(Debug, Clone)]
pub struct Score {
    pub name: String,
    pub content: String,
}

/// Get all embedded scores
pub fn get_all_scores() -> Vec<Score> {
    SCORES
        .iter()
        .map(|(name, content)| Score {
            name: name.to_string(),
            content: content.replace("\\n", "\n"),
        })
        .collect()
}

/// Get a score by name
pub fn get_score(name: &str) -> Option<Score> {
    SCORES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(name, content)| Score {
            name: name.to_string(),
            content: content.replace("\\n", "\n"),
        })
}

/// List all score names
pub fn list_scores() -> Vec<&'static str> {
    SCORES.iter().map(|(name, _)| *name).collect()
}
