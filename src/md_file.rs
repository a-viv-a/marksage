use std::{fs, io, path::PathBuf};

#[derive(Debug)]
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
