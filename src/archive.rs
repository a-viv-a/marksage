use std::path::PathBuf;

use lazy_static::lazy_static;
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

fn is_visible(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with("."))
        .unwrap_or(false)
}

lazy_static! {
    static ref IS_TAGGED_TODO: Regex =
        Regex::new(r"(?s)^(?:\-{3}.*\n\-{3}\n)?\n*(?:#[\w\-/]+\s)*#todo").unwrap();
    static ref PARSE_TODO_ITEMS: Regex = Regex::new(r"(?m)^(\t*)-\s\[(x|\s)]\s.*$").unwrap();
}

#[derive(Debug)]
struct ReadFile {
    path: PathBuf,
    content: String,
}

pub fn archive(vault_path: PathBuf) {
    let walker = WalkDir::new(vault_path).into_iter();

    for readFile in walker
        .filter_entry(is_visible)
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().unwrap_or_default() == "md")
        .map(|e| ReadFile {
            path: e.path().to_path_buf(),
            content: std::fs::read_to_string(e.path()).unwrap(),
        })
        .filter(|f| IS_TAGGED_TODO.is_match(f.content.as_str()))
    {
        println!("{:#?}", entity);
    }
}
