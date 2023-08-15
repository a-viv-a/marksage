use markdown::mdast::{self, Node};
use std::{cell::Cell, path::PathBuf};

use crate::{markdown_file::MdastDocument, util::iterate_markdown_files};

fn archive_markdown(markdown: MdastDocument) -> Option<MdastDocument> {
    println!("mdast: {:#?}", markdown.body);

    let mut new_mdast = markdown.body.children.clone();

    // find or create the archived section
    let archived_section = new_mdast
        .iter()
        .enumerate()
        .find(|(_, node)| match node {
            Node::Heading(heading) => heading.depth == 2 && matches!(heading.children.first(), Some(Node::Text(text)) if text.value == "Archived"),
            _ => false,
        })
        .map(|(index, _)| index)
        .unwrap_or_else(|| {
            let archived_heading = mdast::Heading {
                depth: 2,
                children: vec![Node::Text(mdast::Text {
                    value: "Archived".to_string(),
                    position: None,
                })],
                position: None,
            };
            // find the last list
            let last_list = new_mdast
                .iter()
                .enumerate()
                .rev()
                .find(|(_, node)| matches!(node, Node::List(_)))
                .map(|(index, _)| index + 1)
                .unwrap_or_else(|| new_mdast.len());

            if last_list == new_mdast.len() {
                new_mdast.push(Node::Heading(archived_heading));
            } else {
                new_mdast.insert(last_list, Node::Heading(archived_heading));
            }

            last_list
        });

    // find all the list items that should be archived
    #[derive(Clone)]
    struct DeepIndex(Vec<usize>);
    impl DeepIndex {
        fn append(&self, index: usize) -> Self {
            let mut new = self.clone();
            new.0.push(index);
            new
        }
    }
    let mut should_archive: Vec<DeepIndex> = vec![];
    let mut pending_archive: Vec<DeepIndex> = vec![];
    let mut contains_unmarked: Cell<bool> = Cell::new(false);

    fn explore_vec(
        v: &[Node],
        level: u32,
        index: DeepIndex,
        should_archive: &mut Vec<DeepIndex>,
        pending_archive: &mut Vec<DeepIndex>,
        contains_unmarked: &Cell<bool>,
    ) {
        if level != 0 && contains_unmarked.get() {
            return;
        }
        for (i, node) in v.iter().enumerate() {
            match node {
                Node::List(l) => explore_vec(
                    &l.children,
                    level + 1,
                    index.append(i),
                    should_archive,
                    pending_archive,
                    contains_unmarked,
                ),
                Node::ListItem(li) => match li.checked {
                    Some(true) | None => should_archive.push(index.append(i)),

                    Some(false) => {
                        contains_unmarked.set(true);
                        pending_archive.clear();
                    }
                },
                _ => (),
            }
        }
    }

    pending_archive.append(&mut should_archive);

    Some(markdown.replace_with(
        markdown.frontmatter.clone(),
        mdast::Root {
            children: new_mdast,
            position: None,
        },
    ))
}

pub fn archive(vault_path: PathBuf) {
    iterate_markdown_files(vault_path, "todo")
        .map(MdastDocument::parse)
        .filter_map(archive_markdown)
        .map(|f| f.atomic_overwrite())
        .for_each(Result::unwrap);
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    // macro_rules! test_archive {
    //   ($($name:ident $file:expr => $expected:expr)*) => {
    //     $(
    //         #[test]
    //         fn $name() {
    //             let file = indoc!($file);
    //             let expected = indoc!($expected);
    //             match archive_markdown(file) {
    //                 Some(actual) => assert_eq!(expected, &actual),
    //                 None => assert!(file == expected, "archive_markdown returned None, but the expected output was not the input file. Input was:\n{}", file),
    //             }

    //         }
    //     )*
    //   }
    // }

    // test_archive! {

    //     untouched r#"
    //     - [ ] item 1
    //     "# => r#"
    //     - [ ] item 1
    //     "#

    //     archive_single_item r#"
    //     - [x] item 1
    //     "# => r#"

    //     ## Archived

    //     - [x] item 1
    //     "#

    //     archive_multiple_items r#"
    //     - [x] item 1
    //     - [x] item 2
    //     - [ ] item 3
    //     "# => r#"
    //     - [ ] item 3

    //     ## Archived

    //     - [x] item 1
    //     - [x] item 2
    //     "#

    //     archive_multiple_items_with_sub_items r#"
    //     - [x] item 1
    //         - [x] item 1.1
    //         - [x] item 1.2
    //     - [x] item 2
    //         - [ ] item 2.1
    //     - [ ] item 3
    //     "# => r#"
    //     - [x] item 2
    //         - [ ] item 2.1
    //     - [ ] item 3

    //     ## Archived

    //     - [x] item 1
    //         - [x] item 1.1
    //         - [x] item 1.2
    //     "#
    // }
}
