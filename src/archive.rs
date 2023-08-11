use std::path::PathBuf;

use lazy_static::lazy_static;
use regex::Regex;
use walkdir::WalkDir;

use crate::{markdown, util};

lazy_static! {
    static ref IS_TAGGED_TODO: Regex = util::markdown_contains_tag("todo").unwrap();
    static ref GET_PRE_ARCHIVED_SECTION: Regex = Regex::new(r"(?s)^.*\n## Archived").unwrap();
    static ref PARSE_TODO_ITEMS: Regex = Regex::new(r"(?m)^(\t*)- \[(x| )] .*$\n?").unwrap();
    // The position to insert an archived todo after
    static ref GET_ARCHIVED_TODO_INSERTION_POINT: Regex = Regex::new(r"(?m)^## Archived\n\n").unwrap();
    // The position to insert the ## Archived section after
    static ref GET_ARCHIVED_HEADER_INSERTION_POINT: Regex =
        Regex::new(r"(?s).*\n *- \[(?:x|\s)] .*?(?:$|\n)").unwrap();
}

pub fn archive(vault_path: PathBuf) {
    let walker = WalkDir::new(vault_path).into_iter();

    for markdown in walker
        .filter_entry(util::is_visible)
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().unwrap_or_default() == "md")
        .map(|e| markdown::File::at_path(e.path().to_path_buf()).unwrap())
        .filter(|f| IS_TAGGED_TODO.is_match(f.content.as_str()))
    {
        let pre_archived_section = GET_PRE_ARCHIVED_SECTION
            .find(&markdown.content)
            .map(|m| m.as_str())
            .unwrap_or(markdown.content.as_str());

        println!("pre_archived_section: {}", pre_archived_section);

        let mut marked_tree = false;
        let mut pending_lines = vec![];
        let mut finished_items = vec![];

        for (indent_level, marked, todo) in PARSE_TODO_ITEMS
            .captures_iter(pre_archived_section)
            .map(|caps| {
                // i == 0 is guaranteed to be non none
                let full_match = caps.get(0).unwrap();
                let (_, [indent, mark]) = caps.extract();
                (
                    if indent.is_empty() || indent.starts_with('\t') {
                        indent.len()
                    } else {
                        indent.len() / 4
                    },
                    mark != " ",
                    markdown::Section::from_match(full_match),
                )
            })
        {
            // only put todo items into the archive if the root item is marked along with all of its children
            if indent_level == 0 {
                marked_tree = marked;
                if !pending_lines.is_empty() {
                    finished_items.append(pending_lines.as_mut());
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

        if !pending_lines.is_empty() {
            finished_items.append(pending_lines.as_mut());
        }

        let mut modified_file = markdown::Changes::on(markdown);

        GET_ARCHIVED_TODO_INSERTION_POINT
            .find(modified_file.get_content())
            .map(markdown::Position::after_match)
            .map(|insertion_position| {
                finished_items.iter().for_each(|item| {
                    modified_file.cut_and_paste(*item, insertion_position);
                });

                modified_file.apply();
            })
            .or_else(|| {
                GET_ARCHIVED_HEADER_INSERTION_POINT
                    .find(modified_file.get_content())
                    .map(markdown::Position::after_match)
                    .map(|insertion_position| {
                        modified_file.insert("\n## Archived\n\n", insertion_position);

                        finished_items.iter().for_each(|item| {
                            modified_file.cut_and_paste(*item, insertion_position);
                        });

                        modified_file.apply();
                    })
            });
    }
}
