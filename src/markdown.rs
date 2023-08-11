use std::{collections::BTreeMap, fs, io, path::PathBuf};

pub struct File {
    path: PathBuf,
    pub content: String,
}

impl File {
    pub fn at_path(path: PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(&path)?;
        Ok(Self { path, content })
    }
}

#[derive(Debug)]
pub struct Section {
    start: usize,
    end: usize,
}

impl Section {
    pub fn from_match(m: regex::Match) -> Self {
        Self {
            start: m.start(),
            end: m.end(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Position {
    at: usize,
}

impl Position {
    pub fn after_match(m: regex::Match) -> Self {
        Self { at: m.end() }
    }
}

enum Change<'a> {
    CutPaste(Section, Position),
    Insert(&'a str, Position),
}

pub struct Changes<'a> {
    target_path: PathBuf,
    content: String,
    changes: Vec<Change<'a>>,
}

enum Operation<'a> {
    Add(&'a str),
    Remove(usize),
}

// this is overly complicated, but it's a very fun exercise
impl Changes<'_> {
    pub fn on(file: File) -> Self {
        Self {
            target_path: file.path,
            content: file.content,
            changes: Vec::new(),
        }
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }

    pub fn cut_and_paste(&mut self, from: Section, to: Position) {
        self.changes.push(Change::CutPaste(from, to));
    }

    pub fn insert(&mut self, content: &'static str, at: Position) {
        self.changes.push(Change::Insert(content, at));
    }

    fn compute_new_content(&self) -> String {
        // FIXME: should be usize: vec![Operation]
        let mut operations = BTreeMap::new();

        for change in &self.changes {
            match change {
                Change::CutPaste(section, position) => {
                    let content = &self.content[section.start..section.end];
                    operations.insert(
                        section.start,
                        Operation::Remove(section.end - section.start),
                    );
                    operations.insert(position.at, Operation::Add(content));
                }
                Change::Insert(content, position) => {
                    operations.insert(position.at, Operation::Add(content));
                }
            }
        }

        let mut new_content = String::new();

        let mut last = 0;

        for (at, content) in operations {
            assert!(at <= self.content.len());

            new_content.push_str(&self.content[last..at]);
            match content {
                Operation::Add(content) => {
                    new_content.push_str(content);
                    last = at;
                }
                Operation::Remove(len) => {
                    last = at + len;
                }
            }
        }

        new_content.push_str(&self.content[last..]);

        new_content
    }

    /// Atomically write the changes to the file
    pub fn apply(&self) -> io::Result<()> {
        let mut tmp_path = self.target_path.clone();
        tmp_path.set_extension("tmp.md");
        fs::write(&tmp_path, self.compute_new_content())?;
        fs::rename(tmp_path, &self.target_path)?;
        Ok(())
    }
}
