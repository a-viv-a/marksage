use std::{fmt::format, fs, io, path::PathBuf};

use markdown::mdast::{self, Node};

use lazy_static::lazy_static;
use regex::Regex;

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

pub struct MdastDocument {
    pub frontmatter: Option<String>,
    pub body: mdast::Root,
    path: PathBuf,
}

lazy_static! {
    static ref FRONTMATTER: Regex = Regex::new(r"(?s)-{3}\n(.*)\n-{3}\n").unwrap();
}

macro_rules! mdast_string {
    (|n|) => {
        |n| mdast_string(n, &Context::None)
    };
    (|n, c|) => {
        |(n, c)| mdast_string(n, c)
    };
    (|n, &c|) => {
        |(n, c)| mdast_string(n, &c)
    };
    ($node:expr) => {
        mdast_string($node, &Context::None)
    };
    ($node:expr, $context:expr) => {
        mdast_string($node, &$context)
    };
}

impl MdastDocument {
    /// Produce an ast and frontmatter from a markdown file
    pub fn parse(file: File) -> MdastDocument {
        // mdast doesn't support frontmatter, so we have to extract it manually

        let frontmatter = FRONTMATTER
            .captures(&file.content)
            .map(|c| c.get(1).unwrap().as_str().to_string());

        let body = markdown::to_mdast(
            &file.content[frontmatter.as_ref().map_or(0, |f| f.len() + 10)..],
            &markdown::ParseOptions::gfm(),
        )
        .expect("never fails with gfm");

        match body {
            Node::Root(body) => MdastDocument {
                frontmatter,
                body,
                path: file.path,
            },
            _ => panic!("expected root node, got {:?}", body),
        }
    }

    pub fn atomic_overwrite(self) -> io::Result<()> {
        let mut tmp_path = self.path.clone();
        tmp_path.set_extension("tmp.md");
        fs::write(&tmp_path, self.render())?;
        fs::rename(tmp_path, self.path)?;
        Ok(())
    }

    pub fn render(&self) -> String {
        format!(
            "{}{}",
            self.frontmatter
                .as_ref()
                .map_or_else(String::new, |f| format!("---\n{}\n---\n\n", f)),
            self.body
                .children
                .iter()
                .map(mdast_string!(|n|))
                // handles root level html
                .map(|s| format!("{}{}", s, if s.ends_with('\n') { "" } else { "\n" }))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

fn indent(li: &mdast::ListItem) -> usize {
    li.position
        .as_ref()
        .map(|p| {
            if p.start.column == 1 {
                0
            } else {
                // this makes no sense, but works with the parser
                // it's probably a bug in the parser
                // because the column is 1-indexed, this is like adding 2
                (p.start.column + 1) / 4
            }
        })
        .unwrap_or_default()
}

fn count_longest_sequential_chars(s: &str, c: char) -> usize {
    let mut longest = 0;
    let mut count = 0;

    for ch in s.chars() {
        if ch == c {
            count += 1;
        } else {
            longest = longest.max(count);
            count = 0;
        }
    }

    longest
}

enum Context {
    ListIndex(u32),
    None,
}

fn recursive_mdast_string(nodes: &[Node], sep: &str) -> String {
    nodes
        .iter()
        .map(mdast_string!(|n|))
        .collect::<Vec<String>>()
        .join(sep)
}

fn recursive_contextual_mdast_string<'a>(
    nodes: impl IntoIterator<Item = (&'a Node, Context)>,
) -> String {
    nodes
        .into_iter()
        .map(mdast_string!(|n, &c|))
        .collect::<Vec<String>>()
        .join("")
}

macro_rules! format_mdast {
    (sep=$sep:expr; s = $mdast:expr, $template:expr, $($arg:expr),*) => {
        format!($template, $($arg),*, s = recursive_mdast_string($mdast, $sep))
    };
    (sep=$sep:expr; $mdast:expr, $template:expr) => {
        format!($template, recursive_mdast_string($mdast, $sep))
    };
    (sep=$sep:expr; $mdast:expr) => {
        recursive_mdast_string($mdast, $sep)
    };
    ($($tail:tt)+) => {
        format_mdast!(sep=""; $($tail)*)
    };
}

fn mdast_string(node: &Node, context: &Context) -> String {
    match node {
        Node::Root(_) => format_mdast!(node.children().unwrap()),
        Node::Heading(heading) => {
            format!(
                "{} {}\n",
                "#".repeat(heading.depth as usize),
                format_mdast!(node.children().unwrap())
            )
        }
        Node::Text(t) => t.value.clone(),
        Node::Paragraph(p) => format_mdast!(&p.children, "{}\n"),
        Node::List(l) => match l.start {
            None => format_mdast!(&l.children),
            Some(start) => {
                let mut i = start;
                let mut inc = || {
                    let old = i;
                    i += 1;
                    old
                };
                recursive_contextual_mdast_string(l.children.iter().map(|n| match n {
                    Node::ListItem(_) => (n, Context::ListIndex(inc())),
                    _ => (n, Context::None),
                }))
            }
        },
        Node::ListItem(li) => format!(
            "{}{} {}{}",
            " ".repeat(indent(li) * 4),
            match context {
                Context::ListIndex(i) => format!("{}.", i),
                Context::None => "-".to_string(),
            },
            match li.checked {
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            },
            format_mdast!(&li.children)
        ),
        Node::Code(c) => format!(
            "```{}\n{}\n```\n",
            c.lang.as_ref().unwrap_or(&String::new()),
            c.value
        ),
        Node::InlineCode(c) => {
            let backtick = "`".repeat(count_longest_sequential_chars(&c.value, '`') + 1);
            format!("{}{}{}", backtick, c.value, backtick)
        }
        Node::Emphasis(e) => format_mdast!(&e.children, "*{}*"),
        Node::Strong(s) => format_mdast!(&s.children, "**{}**"),
        Node::Link(l) => {
            let text = format_mdast!(&l.children);
            if l.url == text {
                format!("<{}>", text)
            } else {
                format!("[{}]({})", text, l.url)
            }
        }
        Node::Image(i) => format!("![{}]({})", i.alt, i.url),
        Node::BlockQuote(b) => format_mdast!(&b.children)
            .lines()
            .map(|l| format!("> {}\n", l))
            .collect::<Vec<String>>()
            .join(""),
        Node::ThematicBreak(_) => "---\n".to_string(),
        Node::Html(h) => h.value.clone(),
        Node::FootnoteReference(f) => format!("[^{}]", f.identifier),
        Node::FootnoteDefinition(f) => {
            // FIXME: this would fail if the footnote contains a list
            format_mdast!(sep = "\n    "; s = &f.children, "[^{}]: {s}", f.identifier)
        }
        Node::Table(t) => {
            let mut s = String::new();
            let mut longest = vec![0; t.align.len()];
            for row in &t.children {
                if let Node::TableRow(r) = row {
                    for (i, cell) in r.children.iter().enumerate() {
                        if let Node::TableCell(c) = cell {
                            longest[i] = longest[i].max(format_mdast!(&c.children).len());
                        }
                    }
                }
            }
            let delim = &format!(
                "| {} |\n",
                longest
                    .iter()
                    .map(|l| "-".repeat(*l))
                    .collect::<Vec<String>>()
                    .join(" | ")
            );
            for (row_index, row) in t.children.iter().enumerate() {
                if row_index == 1 {
                    s += delim
                }
                if let Node::TableRow(r) = row {
                    for (column_index, cell) in r.children.iter().enumerate() {
                        if let Node::TableCell(c) = cell {
                            s += &format!(
                                "| {}{} ",
                                format_mdast!(&c.children),
                                " ".repeat(
                                    longest[column_index] - format_mdast!(&c.children).len()
                                )
                            );
                        }
                    }
                    s += "|\n";
                }
            }
            // ensure empty table keep the delim
            if t.children.len() == 1 {
                s += delim
            }
            s
        }
        _ => panic!("Unexpected node type {:#?}", node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq as pretty_assert_eq;

    macro_rules! test_mdast_to_markdown {
        ($($name:ident $input:expr $(=> $expected:expr)?)*) => {
            $(
                #[test]
                #[allow(unused_variables)]
                fn $name() {
                    let input = indoc!($input);
                    let expected: Option<String> = None;
                    $(let expected =
                        Some(indoc!($expected));
                    )?
                    let mdast_document = MdastDocument::parse(File {
                        path: PathBuf::from(""),
                        content: input.to_string(),
                    });
                    let render = mdast_document.render();
                    match expected {
                        Some(expected) => {
                            println!("input:\n{}\nexpected:\n{}\nactual:\n{}", input, expected, render);
                            pretty_assert_eq!(&expected, &render, "expected (left) did not match rendered markdown (right). input ast:\n{:#?}\n\ntest: {}\nexpected / render", mdast_document.body, stringify!($name));
                        }
                        None => {
                            println!("expected:\n{}\nactual:\n{}", input, render);
                            pretty_assert_eq!(input, &render, "input (left) did not match rendered markdown (right). ast:\n{:#?}\n\ntest: {}\ninput / render", mdast_document.body, stringify!($name));
                        }
                    }
                }
            )*
        }
    }

    test_mdast_to_markdown! {
        mdast_simple_file r#"
        # Heading

        - [ ] item 1
        - [x] item 2
        "#

        mdast_nested_list r#"
        - [ ] item 1
            - [ ] item 1.1
            - [x] item 1.2
        - [x] item 2
        "#

        mdast_deep_nested_list r#"
        - [ ] item 1
            - [ ] item 1.1
                - [ ] item 1.1.1
                - [x] item 1.1.2
                - item
            - [x] item 1.2
        "#

        mdast_multiple_headers r#"
        # Heading 1

        - [ ] item 1

        ## Heading 2

        some text
        "#

        mdast_code_block r#"
        # Heading

        ```rust
        fn main() {
            println!("Hello, world!");
        }
        ```

        some text

        Here is some `inline code`.

        Here is more with ```a `` backticks inside```.
        "#

        mdast_lists r#"
        # Heading

        - item 1
        - item 2
        "#

        mdast_numbered_list r#"
        1. First
        2. Second
        "#

        mdast_mixed_lists r#"
        1. First
        2. Second
            - item 1
            - item 2
        3. Third

        - item 1
        "#

        mdast_links r#"
        [Google](https://www.google.com)
        ![Image](https://via.placeholder.com/150)
        ![](https://via.placeholder.com/150)
        "#

        mdast_headers_and_code r#"
        ## Headers & Code

        ### Header 3

        #### Header 4

        ###### Header 5

        ```python
        print("Code block")
        ```
        "#

        mdast_block_quote_thematic_break r#"
        > Quote
        > Multi-line.

        ---

        > Quote
        "#

        mdast_html r#"
        <p>HTML block</p>
        <div style="color: blue;">
            Raw <strong>HTML</strong>
        </div>

        normal text

        <b>HTML</b>
        "#

        mdast_table r#"
        | Header | Header |
        | ------ | ------ |
        | Cell   | Cell   |
        "#

        mdast_table_with_formatting r#"
        | Header | Header |
        | --- | --- |
        | *Cell* | **Cell** |
        | Cell | Cell |
        "# => r#"
        | Header | Header   |
        | ------ | -------- |
        | *Cell* | **Cell** |
        | Cell   | Cell     |
        "#

        mdast_table_with_unicode r#"
        | Foo | Bar |
        | --- | --- |
        | ƒoo | bar |
        "#

        mdast_table_with_alignment r#"
        | Left | Center | Right |
        | :--- | :----: | ----: |
        | foo  |  bar   |   baz |
        "#

        mdast_table_with_partial_alignment r#"
        | Left | None |
        | :--- | ---- |
        | foo  | bar  |
        "#

        mdast_table_with_zero_data r#"
        | Header | Header |
        | ------ | ------ |
        "#

        mdast_auto_links r#"
        <https://www.google.com>
        <mailto:test@example.com>
        "#

        mdast_footnotes r#"
        Here is a footnote reference,[^1] and another.[^long]

        This one is [^super]!

        [^1]: Here is the footnote.

        [^long]: Here's one with multiple blocks.

            Stuff here.

        [^super]: Here's one with multiple blocks.

            Stuff here.

            More stuff here.
        "#

        mdast_frontmatter r#"
        ---
        title: "Hello, world!"
        ---

        # Heading

        some content
        "#
    }

    macro_rules! test_indent {
        ($($name:ident $file:expr, $($indentations:expr)+,)*) => {
            fn test_iter (items: &[Node], indentations: &Vec<usize>) {
                for item in items.iter() {
                    match item {
                        Node::ListItem(li) => {
                            let indent = indent(li);
                            let expected_indent = indentations.get(li.position.as_ref().unwrap().start.line - 1).unwrap();
                            println!("{}: i{} == ex{}", li.position.as_ref().unwrap().start.line, indent, expected_indent);
                            assert_eq!(indent, *expected_indent, "expected indent {} but got {} for item position {:#?}", expected_indent, indent, li.position.as_ref().unwrap().start.column);
                            test_iter(&li.children, indentations);
                        },
                        Node::List(l) => test_iter(&l.children, indentations),
                        _ => (),
                    }
                }
            }
            $(
                #[test]
                fn $name() {
                    let file = indoc!($file);
                    let ast = markdown::to_mdast(file, &markdown::ParseOptions::gfm()).expect("never fails with gfm");
                    let list = ast.children().unwrap().iter().next().unwrap();
                    let list = match list {
                        Node::List(l) => l,
                        _ => panic!("expected list, got {:#?}", list),
                    };
                    let items = &list.children;
                    let indentations = vec![$($indentations,)+];

                    test_iter(items, &indentations);
                }
            )*
        }
    }

    test_indent! {
        indent_no_indentation r#"
        - item 1
        - item 2
        "#, 0 0,

        indent_one_level r#"
        - item 1
            - item 1.1
        - item 2
        "#, 0 1 0,

        indent_many_levels r#"
        - item 1
            - item 1.1
                - item 1.1.1
                    - item 1.1.1.1
                        - item
                            - item
        - item 2
        "#, 0 1 2 3 4 5 0,

        index_with_tabs r#"
        - item 1
        \t- item 1.1
        \t\t- item 1.1.1
        - item 2
        "#, 0 1 2 0,
    }
}
