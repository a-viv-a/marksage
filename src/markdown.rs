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

#[derive(Debug, Copy, Clone)]
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

#[derive(Copy, Clone)]
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
        let mut operations: BTreeMap<usize, Vec<Operation<'_>>> = BTreeMap::new();

        let mut insert_operation = |key, operation| {
            operations
                .entry(key)
                .and_modify(|vec| vec.push(operation))
                .or_insert_with(|| vec![operation]);
        };

        for change in &self.changes {
            match change {
                Change::CutPaste(section, position) => {
                    let content = &self.content[section.start..section.end];
                    insert_operation(
                        section.start,
                        Operation::Remove(section.end - section.start),
                    );
                    insert_operation(position.at, Operation::Add(content));
                }
                Change::Insert(content, position) => {
                    insert_operation(position.at, Operation::Add(content));
                }
            }
        }

        let mut new_content = String::new();

        let mut last = 0;

        for (at, positional_operations) in operations {
            assert!(
                at <= self.content.len(),
                "at: {}, len: {} is invalid, at must be <= len",
                at,
                self.content.len()
            );
            assert!(
                at >= last,
                "at: {}, last: {} is invalid, at must be >= last",
                at,
                last
            );

            let mut deletion_offset = 0;

            new_content.push_str(&self.content[last..at]);
            positional_operations
                .iter()
                .for_each(|operation| match operation {
                    Operation::Add(content) => new_content.push_str(content),
                    Operation::Remove(len) => {
                        new_content.truncate(new_content.len() - len);
                        deletion_offset = *len;
                    }
                });

            if deletion_offset > 0 {
                assert!(
                    positional_operations.len() == 1,
                    "a position with a deletion should only have one operation"
                );
            }

            last = at + deletion_offset;
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