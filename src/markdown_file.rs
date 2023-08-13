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
                .map(mdast_string)
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

fn recursive_mdast_string(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(mdast_string)
        .collect::<Vec<String>>()
        .join("")
}

fn mdast_string(node: &Node) -> String {
    match node {
        Node::Root(_) => recursive_mdast_string(node.children().unwrap()),
        Node::Heading(heading) => {
            format!(
                "{} {}\n",
                "#".repeat(heading.depth as usize),
                recursive_mdast_string(node.children().unwrap())
            )
        }
        Node::Text(t) => t.value.clone(),
        Node::Paragraph(p) => format!("{}\n", recursive_mdast_string(&p.children)),
        Node::List(l) => recursive_mdast_string(&l.children),
        Node::ListItem(li) => format!(
            "{}- {}{}",
            " ".repeat(indent(li) * 4),
            match li.checked {
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            },
            recursive_mdast_string(&li.children)
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

        // this section needs work
        Node::Emphasis(e) => format!("*{}*", recursive_mdast_string(&e.children)),
        Node::Strong(s) => format!("**{}**", recursive_mdast_string(&s.children)),
        Node::Link(l) => format!("[{}]({})", recursive_mdast_string(&l.children), l.url),
        Node::Image(i) => format!("![{}]({})", i.alt, i.url),
        // needs to insert > at the start of each line
        Node::BlockQuote(b) => recursive_mdast_string(&b.children)
            .lines()
            .map(|l| format!("> {}\n", l))
            .collect::<Vec<String>>()
            .join(""),
        Node::ThematicBreak(_) => "---\n".to_string(),
        Node::Html(h) => h.value.clone(),
        Node::Table(t) => "TODO: table".to_string(),
        _ => panic!("Unexpected node type {:#?}", node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq as pretty_assert_eq;

    macro_rules! test_mdast_to_markdown {
        ($($name:ident $file:expr)*) => {
            $(
                #[test]
                fn $name() {
                    let file = indoc!($file);
                    let mdast_document = MdastDocument::parse(File {
                        path: PathBuf::from(""),
                        content: file.to_string(),
                    });
                    let render = mdast_document.render();
                    println!("expected:\n{}\nactual:\n{}", file, render);
                    pretty_assert_eq!(file, &render, "input file (left) did not match rendered markdown (right). ast:\n{:#?}\n\ntest: {}", mdast_document.body, stringify!($name));
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

        mdast_emphasis_and_lists r#"
        *Italic*, **bold**, ***both***.

        1. First
        2. Second

        - Nested 1
        - Nested 2
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

        mdast_footnotes_and_auto_links r#"
        ## Footnotes & Auto Links

        Footnote[^1].
        <https://www.example.com>

        ## References & Raw HTML

        [Ref][1]
        [1]: https://www.example.com
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
