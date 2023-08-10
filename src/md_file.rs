use std::{fs, io, path::PathBuf};

pub struct Markdown {
    path: PathBuf,
    pub content: String,
}

impl Markdown {
    pub fn at_path(path: PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(&path)?;
        Ok(Self { path, content })
    }
}

#[derive(Debug)]
pub struct MarkdownSection {
    start: usize,
    end: usize,
}

impl MarkdownSection {
    pub fn from_match(m: regex::Match) -> Self {
        Self {
            start: m.start(),
            end: m.end(),
        }
    }
}
