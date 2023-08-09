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

fn contains_tag(tag: &str) -> Result<Regex, regex::Error> {
    Regex::new(
        format!(
            r"(?sx)^
        (?:                 # match the optional frontmatter section
            \n*                 # leading newlines
            \-{{3}}             # frontmatter starts with `---`
            .*\n                # frontmatter content  
            \-{{3}}\n           # frontmatter ends with `---\n`
        )?
        \n*                 # match leading newlines
        (?:\#[\w\-/]+\s)*   # match other tags
        \#{tag}             # match the arbitrary tag"
        )
        .as_str(),
    )
}

lazy_static! {
    static ref IS_TAGGED_TODO: Regex = contains_tag("todo").unwrap();
    static ref PARSE_TODO_ITEMS: Regex = Regex::new(r"(?m)^(\t*)-\s\[(x|\s)]\s.*$").unwrap();
}

#[derive(Debug)]
struct ReadFile {
    path: PathBuf,
    content: String,
}

pub fn archive(vault_path: PathBuf) {
    let walker = WalkDir::new(vault_path).into_iter();

    for read_file in walker
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
        let mut marked_tree = false;
        let mut pending_lines = vec![];
        let mut finished_items = vec![];

        for (indent_level, marked, todo) in PARSE_TODO_ITEMS
            .captures_iter(read_file.content.as_str())
            .map(|caps| {
                let (todo, [indent, mark]) = caps.extract();
                (indent.len(), mark != " ", todo)
            })
        {
            // only put todo items into the archive if the root item is marked along with all of its children
            if indent_level == 0 {
                marked_tree = marked;
                if !pending_lines.is_empty() {
                    finished_items.push(pending_lines.join("\n"));
                    pending_lines.clear();
                }
            }

            if marked_tree {
                if marked {
                    pending_lines.push(todo);
                } else {
                    pending_lines.clear();
                    marked_tree = false;
                }
            }
        }

        println!("{:#?}", finished_items);
    }
}
