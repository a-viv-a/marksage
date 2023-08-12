use std::path::PathBuf;

use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    markdown_file::{self, MdastDocument},
    util::iterate_markdown_files,
};

lazy_static! {
    static ref GET_PRE_ARCHIVED_SECTION: Regex = Regex::new(r"(?s)^.*\n## Archived").unwrap();
    static ref PARSE_TODO_ITEMS: Regex = Regex::new(r"(?m)^(\t*)- \[(x| )] .*$\n?").unwrap();
    // The position to insert an archived todo after
    static ref GET_ARCHIVED_TODO_INSERTION_POINT: Regex = Regex::new(r"(?m)^## Archived\n\n").unwrap();
    // The position to insert the ## Archived section after
    static ref GET_ARCHIVED_HEADER_INSERTION_POINT: Regex =
        Regex::new(r"(?s)^(?:.*\n *)?- \[(?:x|\s)] (.*?)(?:$|\n)").unwrap();
}

fn archive_markdown(markdown: MdastDocument) -> Option<MdastDocument> {
    println!("{:?}", markdown.body);

    None
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
