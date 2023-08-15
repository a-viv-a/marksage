use markdown::mdast::{self, Node};
use std::{cell::Cell, path::PathBuf};

use crate::{markdown_file::MdastDocument, util::iterate_markdown_files};

fn archive_markdown(markdown: MdastDocument) -> Option<MdastDocument> {
    // println!("mdast: {:#?}", markdown.body);

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

    /*
    mdast: Root {
        children: [
            Paragraph {
                children: [
                    Text {
                        value: "#todo",
                        position: Some(
                            1:1-1:6 (0-5),
                        ),
                    },
                ],
                position: Some(
                    1:1-1:6 (0-5),
                ),
            },
            List {
                children: [
                    ListItem {
                        children: [
                            Paragraph {
                                children: [
                                    Text {
                                        value: "todo0",
                                        position: Some(
                                            3:7-3:12 (13-18),
                                        ),
                                    },
                                ],
                                position: Some(
                                    3:7-3:12 (13-18),
                                ),
                            },
                        ],
                        position: Some(
                            3:1-3:12 (7-18),
                        ),
                        spread: false,
                        checked: Some(
                            true,
                        ),
                    },
                    ListItem {
                        children: [
                            Paragraph {
                                children: [
                                    Text {
                                        value: "todo1",
                                        position: Some(
                                            4:7-4:12 (25-30),
                                        ),
                                    },
                                ],
                                position: Some(
                                    4:7-4:12 (25-30),
                                ),
                            },
                            List {
                                children: [
                                    ListItem {
                                        children: [
                                            Paragraph {
                                                children: [
                                                    Text {
                                                        value: "todo1-1",
                                                        position: Some(
                                                            5:11-5:18 (41-48),
                                                        ),
                                                    },
                                                ],
                                                position: Some(
                                                    5:11-5:18 (41-48),
                                                ),
                                            },
                                        ],
                                        position: Some(
                                            5:3-5:18 (33-48),
                                        ),
                                        spread: false,
                                        checked: Some(
                                            false,
                                        ),
                                    },
                                    ListItem {
                                        children: [
                                            Paragraph {
                                                children: [
                                                    Text {
                                                        value: "todo1-2",
                                                        position: Some(
                                                            6:11-6:18 (59-66),
                                                        ),
                                                    },
                                                ],
                                                position: Some(
                                                    6:11-6:18 (59-66),
                                                ),
                                            },
                                        ],
                                        position: Some(
                                            6:3-6:18 (51-66),
                                        ),
                                        spread: false,
                                        checked: Some(
                                            false,
                                        ),
                                    },
                                ],
                                position: Some(
                                    5:3-6:18 (33-66),
                                ),
                                ordered: false,
                                start: None,
                                spread: false,
                            },
                        ],
                        position: Some(
                            4:1-6:18 (19-66),
                        ),
                        spread: false,
                        checked: Some(
                            true,
                        ),
                    },
                    ListItem {
                        children: [
                            Paragraph {
                                children: [
                                    Text {
                                        value: "todo2",
                                        position: Some(
                                            7:7-7:12 (73-78),
                                        ),
                                    },
                                ],
                                position: Some(
                                    7:7-7:12 (73-78),
                                ),
                            },
                        ],
                        position: Some(
                            7:1-7:12 (67-78),
                        ),
                        spread: false,
                        checked: Some(
                            false,
                        ),
                    },
                    ListItem {
                        children: [
                            Paragraph {
                                children: [
                                    Text {
                                        value: "todo3",
                                        position: Some(
                                            8:7-8:12 (85-90),
                                        ),
                                    },
                                ],
                                position: Some(
                                    8:7-8:12 (85-90),
                                ),
                            },
                        ],
                        position: Some(
                            8:1-9:1 (79-91),
                        ),
                        spread: false,
                        checked: Some(
                            false,
                        ),
                    },
                ],
                position: Some(
                    3:1-9:1 (7-91),
                ),
                ordered: false,
                start: None,
                spread: false,
            },
            Heading {
                children: [
                    Text {
                        value: "Archived",
                        position: Some(
                            10:4-10:12 (95-103),
                        ),
                    },
                ],
                position: Some(
                    10:1-10:12 (92-103),
                ),
                depth: 2,
            },
            Paragraph {
                children: [
                    Text {
                        value: "some random p content",
                        position: Some(
                            12:1-12:22 (105-126),
                        ),
                    },
                ],
                position: Some(
                    12:1-12:22 (105-126),
                ),
            },
        ],
        position: Some(
            1:1-13:1 (0-127),
        ),
    }

        */

    fn should_archive(node: &Node) -> bool {
        match node {
            Node::ListItem(list_item) => match list_item.checked {
                Some(true) | None => list_item.children.iter().all(should_archive),
                Some(false) => false,
            },
            Node::List(list) => list.children.iter().all(should_archive),
            _ => true,
        }
    }

    for (i, node) in markdown
        .body
        .children
        .iter()
        .take(archived_section)
        .enumerate()
    {
        if let Node::List(list) = node {
            let mut archived_list = list.clone();
            archived_list.children = list
                .children
                .iter()
                .filter_map(|node| match node {
                    Node::ListItem(list_item) if should_archive(node) => {
                        Some(Node::ListItem(list_item.clone()))
                    }
                    _ => None,
                })
                .collect();

            if !archived_list.children.is_empty() {
                new_mdast.insert(archived_section + 1, Node::List(archived_list));
            }
        }
    }

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
