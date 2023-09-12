use markdown::mdast::{self, Node};
use rayon::iter::ParallelIterator;
use std::path::PathBuf;

use crate::{markdown_file::MdastDocument, util::iterate_tagged_markdown_files};

fn archive_mdast(mdast: &mdast::Root) -> Option<mdast::Root> {
    enum Assessment {
        Is(bool),
        Maybe,
    }

    impl Assessment {
        fn bias(self, by: Assessment) -> Self {
            match (self, by) {
                (Assessment::Is(false), _) | (_, Assessment::Is(false)) => Assessment::Is(false),
                (Assessment::Is(true), _) | (_, Assessment::Is(true)) => Assessment::Is(true),
                _ => Assessment::Maybe,
            }
        }

        fn definitively(self) -> bool {
            matches!(self, Assessment::Is(true))
        }
    }

    // using collect is fine for performance because iter is lazy
    // short circuiting is achieved bc next stops being called on first false
    impl FromIterator<Assessment> for Assessment {
        fn from_iter<T: IntoIterator<Item = Assessment>>(iter: T) -> Self {
            let mut result = Assessment::Maybe;
            for next in iter {
                result = result.bias(next);
                if matches!(result, Assessment::Is(false)) {
                    return result;
                }
            }
            result
        }
    }

    fn should_archive(node: &Node) -> Assessment {
        match node {
            Node::ListItem(list_item) => match list_item.checked {
                Some(true) => list_item
                    .children
                    .iter()
                    .map(should_archive)
                    .collect::<Assessment>()
                    .bias(Assessment::Is(true)),
                None => list_item
                    .children
                    .iter()
                    .map(should_archive)
                    .collect::<Assessment>(),
                Some(false) => Assessment::Is(false),
            },
            Node::List(list) => list
                .children
                .iter()
                .map(should_archive)
                .collect::<Assessment>(),
            _ => Assessment::Maybe,
        }
    }

    let mut new_mdast: Vec<Node> = mdast.children.clone();

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
                .map_or_else(|| new_mdast.len(), |(index, _)| index + 1);

            if last_list == new_mdast.len() {
                new_mdast.push(Node::Heading(archived_heading));
            } else {
                new_mdast.insert(last_list, Node::Heading(archived_heading));
            }

            last_list
        });

    let mut to_delete = vec![];
    for (i, node) in mdast.children.iter().take(archived_section).enumerate() {
        if let Node::List(list) = node {
            let archived_children: Vec<_> = list
                .children
                .iter()
                .enumerate()
                .filter_map(|(j, node)| match node {
                    Node::ListItem(list_item) if should_archive(node).definitively() => {
                        Some((j, Node::ListItem(list_item.clone())))
                    }
                    _ => None,
                })
                .collect();

            if archived_children.is_empty() {
                continue;
            }

            for (j, _) in &archived_children {
                to_delete.push((i, *j));
            }

            let mut new_children: Vec<_> = archived_children
                .into_iter()
                .map(|(_, node)| node)
                .collect();

            if new_children.is_empty() {
                continue;
            }

            match new_mdast.get(archived_section + 1) {
                Some(Node::List(list)) => {
                    let mut list = list.clone();
                    new_children.append(&mut list.children);
                    list.children = new_children;
                    new_mdast[archived_section + 1] = Node::List(list);
                }
                _ => {
                    new_mdast.insert(
                        archived_section + 1,
                        Node::List(mdast::List {
                            children: new_children,
                            ..list.clone()
                        }),
                    );
                }
            }
        }
    }

    for (i, j) in to_delete.iter().rev() {
        if let Node::List(list) = &mut new_mdast[*i] {
            let mut new_children = list.children.clone();
            new_children.remove(*j);
            if new_children.is_empty() {
                new_mdast.remove(*i);
            } else {
                list.children = new_children;
            }
        } else {
            panic!("to_delete target should have been a list");
        }
    }

    if to_delete.is_empty() {
        return None;
    }

    Some(mdast::Root {
        children: new_mdast,
        position: None,
    })
}

#[must_use]
pub fn archive(vault_path: PathBuf) -> impl ParallelIterator<Item = (PathBuf, String)> {
    iterate_tagged_markdown_files(vault_path, "todo")
        .map(|file| (file.path, MdastDocument::parse(file.content.as_str())))
        .filter_map(|(path, document)| {
            archive_mdast(&document.body).map(|mdast| {
                (
                    path,
                    MdastDocument {
                        frontmatter: None,
                        body: mdast,
                    }
                    .render(),
                )
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    macro_rules! test_archive {
      ($($name:ident $input:expr => $expected:expr)*) => {
        $(
            #[test]
            fn $name() {
                let input = indoc!($input);
                println!("input: \n{}", input);
                let input_document = MdastDocument::parse(input);
                let expected = indoc!($expected);
                println!("expected: \n{}", expected);
                match archive_mdast(&input_document.body) {
                    Some(actual_mdast) => {
                        let actual = MdastDocument::of(actual_mdast).render();
                        println!("actual: \n{}", actual);
                        assert_eq!(actual, expected);
                        assert_ne!(input, expected);
                    },
                    None => assert_eq!(input, expected, "archive_markdown returned None, but the expected output was not the input file. Input was:\n{}", input),
                }

            }
        )*
      }
    }

    test_archive! {

        untouched r#"
        - [ ] item 1
        "# => r#"
        - [ ] item 1
        "#

        archive_single_item r#"
        - [x] item 1
        "# => r#"
        ## Archived

        - [x] item 1
        "#

        archive_multiple_items r#"
        - [x] item 1
        - [x] item 2
        - [ ] item 3
        "# => r#"
        - [ ] item 3

        ## Archived

        - [x] item 1
        - [x] item 2
        "#

        archive_multiple_items_with_sub_items r#"
        - [x] item 1
            - [x] item 1.1
            - [x] item 1.2
        - [x] item 2
            - [ ] item 2.1
        - [ ] item 3
        "# => r#"
        - [x] item 2
            - [ ] item 2.1
        - [ ] item 3

        ## Archived

        - [x] item 1
            - [x] item 1.1
            - [x] item 1.2
        "#

        archive_mixed_items r#"
        - [x] item 1
            1. [x] item 1.1
            2. [x] item 1.2
        - collection
            - [x] item 2.1
            - [ ] item 2.2
        - second collection
            - [x] item 3.1
            - [x] item 3.2
        - [ ] item 4
        "# => r#"
        - collection
            - [x] item 2.1
            - [ ] item 2.2
        - [ ] item 4

        ## Archived

        - [x] item 1
            1. [x] item 1.1
            2. [x] item 1.2
        - second collection
            - [x] item 3.1
            - [x] item 3.2
        "#

        do_not_archive_non_todo_lists r#"
        - [x] item 1
            1. [x] item 1.1
            2. [x] item 1.2
        - collection
            - stuff
            - more stuff
        "# => r#"
        - collection
            - stuff
            - more stuff
        
        ## Archived

        - [x] item 1
            1. [x] item 1.1
            2. [x] item 1.2
        "#

        archive_root_level_insert r#"
        - [ ] 1
        - [ ] 2
        - [x] 3
            - [x] 3.1
        - [ ] 4
        - [x] 5

        ## Archived

        - [x] a1
        - [x] a2
        - [x] a3
            - [x] a3.1
        - [x] a4
        "# => r#"
        - [ ] 1
        - [ ] 2
        - [ ] 4

        ## Archived

        - [x] 3
            - [x] 3.1
        - [x] 5
        - [x] a1
        - [x] a2
        - [x] a3
            - [x] a3.1
        - [x] a4
        "#
    }
}
