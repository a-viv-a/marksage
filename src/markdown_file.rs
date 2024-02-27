use std::{fs, io, path::PathBuf};

use markdown::{
    mdast::{self, Node},
    Constructs, ParseOptions,
};
use unicode_width::UnicodeWidthStr;

pub struct File {
    pub path: PathBuf,
    pub content: String,
}

impl File {
    pub fn at_path(path: PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(&path)?;
        Ok(Self { path, content })
    }

    pub fn atomic_overwrite(path: &PathBuf, content: String) -> io::Result<()> {
        let tmp_path = path.with_extension(format!(
            "tmp{}{}",
            rand::random::<u64>(),
            path.extension()
                .unwrap_or_default()
                .to_str()
                .map_or_else(String::new, |s| format!(".{s}"))
        ));
        fs::write(&tmp_path, content)?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }
}

pub struct MdastDocument {
    pub root: mdast::Root,
}

impl MdastDocument {
    /// Produce an ast and frontmatter from a markdown string
    pub fn parse(md_string: &str) -> MdastDocument {
        let root = markdown::to_mdast(
            &md_string,
            &ParseOptions {
                constructs: Constructs {
                    math_flow: true,
                    math_text: true,
                    frontmatter: true,
                    ..Constructs::gfm()
                },
                ..ParseOptions::gfm()
            },
        )
        .expect("never fails with gfm");

        match root {
            Node::Root(root) => MdastDocument { root },
            _ => panic!("expected root node, got {root:?}"),
        }
    }
    #[cfg(test)]
    pub fn of(root: mdast::Root) -> MdastDocument {
        MdastDocument { root }
    }

    pub fn render(&self) -> String {
        self.root
            .children
            .iter()
            .map(|n| mdast_string(n, Context::default()))
            // handles root level html
            .map(|s| format!("{}{}", s, if s.ends_with('\n') { "" } else { "\n" }))
            .collect::<Vec<String>>()
            .join("\n")
    }
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

#[derive(Default, Clone, Copy)]
struct Context {
    pub list_index: Option<u32>,
    pub list_indent: Option<usize>,
}

fn recursive_mdast_string(ctx: Context, nodes: &[Node], sep: &str) -> String {
    nodes
        .iter()
        .map(|n| mdast_string(n, ctx))
        .collect::<Vec<String>>()
        .join(sep)
}

fn recursive_contextual_mdast_string<'a>(
    nodes: impl IntoIterator<Item = (&'a Node, Context)>,
) -> String {
    nodes
        .into_iter()
        .map(|(n, ctx)| mdast_string(n, ctx))
        .collect::<String>()
}

macro_rules! format_mdast {
    ($ctx:ident sep=$sep:expr; s = $mdast:expr, $template:expr, $($arg:expr),*) => {
        format!($template, $($arg),*, s = recursive_mdast_string($ctx, $mdast, $sep))
    };
    ($ctx:ident sep=$sep:expr; $mdast:expr, $template:expr) => {
        format!($template, recursive_mdast_string($ctx, $mdast, $sep))
    };
    ($ctx:ident; $($tail:tt)+) => {
        format_mdast!($ctx sep=""; $($tail)*)
    };
}

fn mdast_string(node: &Node, ctx: Context) -> String {
    match node {
        Node::Root(_) => recursive_mdast_string(ctx, node.children().unwrap(), ""),
        Node::Heading(heading) => {
            format!(
                "{} {}\n",
                "#".repeat(heading.depth as usize),
                recursive_mdast_string(ctx, node.children().unwrap(), "")
            )
        }
        Node::Text(t) => t.value.clone(),
        Node::Paragraph(p) => format_mdast!(ctx; &p.children, "{}\n"),
        Node::List(l) => {
            let list_indent = Some(ctx.list_indent.map_or(0, |i| i + 1));
            match l.start {
                None => recursive_mdast_string(
                    Context {
                        list_index: None,
                        list_indent,
                    },
                    &l.children,
                    "",
                ),
                Some(start) => {
                    let mut i = start;
                    let mut inc = || {
                        let old = i;
                        i += 1;
                        old
                    };
                    recursive_contextual_mdast_string(l.children.iter().map(|n| match n {
                        Node::ListItem(_) => (
                            n,
                            Context {
                                list_index: Some(inc()),
                                list_indent,
                            },
                        ),
                        _ => (
                            n,
                            Context {
                                list_index: None,
                                list_indent,
                            },
                        ),
                    }))
                }
            }
        }
        Node::ListItem(li) => format!(
            "{}{} {}{}",
            " ".repeat(ctx.list_indent.unwrap_or(0) * 4),
            match ctx.list_index {
                Some(i) => format!("{i}."),
                None => "-".to_string(),
            },
            match li.checked {
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            },
            recursive_mdast_string(
                Context {
                    list_index: None,
                    ..ctx
                },
                &li.children,
                ""
            )
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
        Node::Emphasis(e) => format_mdast!(ctx; &e.children, "*{}*"),
        Node::Strong(s) => format_mdast!(ctx; &s.children, "**{}**"),
        Node::Delete(d) => format_mdast!(ctx; &d.children, "~~{}~~"),
        Node::Break(_) => "\n".to_string(),
        Node::Link(l) => {
            let text = recursive_mdast_string(ctx, &l.children, "");
            if l.url == text {
                format!("<{text}>")
            } else {
                format!("[{text}]({})", l.url)
            }
        }
        Node::Image(i) => format!("![{}]({})", i.alt, i.url),
        Node::BlockQuote(b) => recursive_mdast_string(ctx, &b.children, "")
            .lines()
            .map(|l| format!("> {l}\n"))
            .collect::<String>(),
        Node::ThematicBreak(_) => "---\n".to_string(),
        Node::Html(h) => h.value.clone(),
        Node::ImageReference(ir) => format!("![{}][{}]", ir.alt, ir.identifier),
        Node::Definition(d) => format!("[{}]: {}", d.identifier, d.url),
        Node::FootnoteReference(f) => format!("[^{}]", f.identifier),
        Node::FootnoteDefinition(f) => {
            format!(
                "[^{}]: {}",
                f.identifier,
                &f.children
                    .iter()
                    .enumerate()
                    .map(|(i, n)| mdast_string(n, ctx)
                        .lines()
                        .map(|l| format!("{}{}\n", if i == 0 { "" } else { "    " }, l))
                        .collect::<String>())
                    .collect::<Vec<String>>()
                    .join("\n")
            )
        }
        Node::Table(t) => {
            let mut s = String::new();
            // A 1d vector of (Cell render, width) pairs, omitting overrun cells
            let mut table_skeleton: Vec<Option<(String, usize)>> =
                vec![None; t.children.len() * t.align.len()];

            let mut longest = t
                .align
                .iter()
                // handle the case when the alignment indicator is the longest thing in the column
                .map(|align| match align {
                    mdast::AlignKind::Left | mdast::AlignKind::Right => 2,
                    mdast::AlignKind::Center => 3,
                    mdast::AlignKind::None => 1,
                })
                .collect::<Vec<usize>>();

            for (row_index, row) in t.children.iter().enumerate() {
                if let Node::TableRow(r) = row {
                    for (column_index, cell) in r.children.iter().enumerate().take(t.align.len()) {
                        if let Node::TableCell(c) = cell {
                            let cell_string = recursive_mdast_string(ctx, &c.children, "");
                            let cell_width = UnicodeWidthStr::width(cell_string.as_str());
                            longest[column_index] = longest[column_index].max(cell_width);
                            table_skeleton[row_index * t.align.len() + column_index] =
                                Some((cell_string, cell_width));
                        }
                    }
                }
            }

            let delim = &format!(
                "| {} |\n",
                longest
                    .iter()
                    .zip(t.align.iter())
                    .map(|(len, align)| match align {
                        // these subtractions wont overflow because the smallest value in longest is AlignKind::*'s min width
                        mdast::AlignKind::Left => format!(":{}", "-".repeat(*len - 1)),
                        mdast::AlignKind::Center => format!(":{}:", "-".repeat(len - 2)),
                        mdast::AlignKind::Right => format!("{}:", "-".repeat(*len - 1)),
                        mdast::AlignKind::None => "-".repeat(*len),
                    })
                    .collect::<Vec<String>>()
                    .join(" | ")
            );

            for (i, cell) in table_skeleton.iter().enumerate() {
                if i != 0 && i <= t.align.len() && i % t.align.len() == 0 {
                    s += delim;
                }
                if let Some((cell_string, cell_width)) = cell {
                    let pad_len = longest[i % t.align.len()] - cell_width;
                    s += &match t.align[i % t.align.len()] {
                        mdast::AlignKind::Left | mdast::AlignKind::None => {
                            format!("| {}{} ", cell_string, " ".repeat(pad_len))
                        }
                        mdast::AlignKind::Right => {
                            format!("| {}{} ", " ".repeat(pad_len), cell_string)
                        }
                        mdast::AlignKind::Center => format!(
                            "| {}{}{} ",
                            " ".repeat((pad_len) / 2),
                            cell_string,
                            " ".repeat((pad_len + 1) / 2)
                        ),
                    };
                }
                if i % t.align.len() == t.align.len() - 1 {
                    s += "|\n";
                }
            }

            // ensure empty table keep the delim
            if t.children.len() == 1 {
                s += delim;
            }
            s
        }
        Node::Math(math) => format!("$$\n{}\n$$", math.value),
        Node::InlineMath(math) => format!("${}$", math.value),
        Node::Yaml(yaml) => format!("---\n{}\n---\n", yaml.value),
        _ => panic!("Unexpected node type {node:#?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq as pretty_assert_eq;
    use proptest::prelude::*;

    macro_rules! test_mdast_to_markdown {
        ($($name:ident $input:expr $(=> $expected:expr)?)*) => {
            $(
                #[test]
                #[allow(unused_variables)]
                fn $name() {
                    let input = indoc!($input);
                    println!("input:\n{}", input);
                    let expected: Option<String> = None;
                    $(let expected =
                        Some(indoc!($expected));
                    )?
                    let mdast_document = MdastDocument::parse(input);
                    let render = mdast_document.render();
                    match expected {
                        Some(expected) => {
                            println!("expected:\n{}\nactual:\n{}", expected, render);
                            pretty_assert_eq!(&expected, &render, "expected (left) did not match rendered markdown (right). input ast:\n{:#?}\n\ntest: {}\nexpected / render", mdast_document.root, stringify!($name));
                        }
                        None => {
                            println!("actual:\n{}", render);
                            pretty_assert_eq!(input, &render, "input (left) did not match rendered markdown (right). ast:\n{:#?}\n\ntest: {}\ninput / render", mdast_document.root, stringify!($name));
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

        mdast_jagged_table_two_columns r#"
        | Header | Header |
        | --- | --- |
        | Cell |
        | Cell | Cell |
        | Cell | Cell | Cell |
        "# => r#"
        | Header | Header |
        | ------ | ------ |
        | Cell   |
        | Cell   | Cell   |
        | Cell   | Cell   |
        "#

        mdast_jagged_table_three_columns r#"
        | H | Hah | H |
        | --- | --- | --- |
        | C |
        | C | C |
        | C | C | C |
        | C | C | C | C |
        "# => r#"
        | H | Hah | H |
        | - | --- | - |
        | C |
        | C | C   |
        | C | C   | C |
        | C | C   | C |
        "#

        mdast_tiny_column_with_alignment_center r#"
        | H | 
        | :-: |
        | C |
        "# => r#"
        |  H  |
        | :-: |
        |  C  |
        "#

        mdast_tiny_column_with_alignment_right r#"
        | H |
        | -: |
        | C |
        "# => r#"
        |  H |
        | -: |
        |  C |
        "#

        mdast_tiny_column_with_alignment_left r#"
        | H |
        | :- |
        | C |
        "# => r#"
        | H  |
        | :- |
        | C  |
        "#

        mdast_table_with_empty_columns r#"
        | H | | H |
        | - | - | - |
        | C | | C |
        "# => r#"
        | H |   | H |
        | - | - | - |
        | C |   | C |
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

            1. Evil list
            2. (=
        "#

        mdast_image_reference r#"
        ![Image][1]

        [1]: https://via.placeholder.com/150
        "#

        mdast_frontmatter r#"
        ---
        title: "Hello, world!"
        number: 1
        ---

        # Heading

        some content
        "#

        mdast_horizontal_rule r#"
        stuff

        ---

        more stuff
        "#

        mdast_dual_horizontal_rule r#"
        stuff

        ---

        ---

        more stuff
        "#

        mdast_many_horizontal_rules r#"
        # rules!
    
        ---

        ***

        ___
        "# => r#"
        # rules!

        ---

        ---

        ---
        "#

        mdast_mathjax r#"
        $$
        \begin{aligned}
        \dot{x} & = \sigma(y-x) \\
        \dot{y} & = \rho x - y - xz \\
        \dot{z} & = -\beta z + xy
        \end{aligned}
        $$
        "#
    }

    proptest! {
        #[test]
        fn mdast_document_render_does_not_crash(input in "[[:alpha:]0-9#!<>`\\-\\*_~\\$\\n\\[\\] ]{10,}") {
            let mdast_document = MdastDocument::parse(&input);
            mdast_document.render();
        }

        #[test]
        fn mdast_document_repeated_render_is_equal(input in "[[:alpha:]0-9#!<>`\\-\\*_~\\$\\n\\[\\] ]{10,}") {
            let render = MdastDocument::parse(&input).render();
            let render2 = MdastDocument::parse(&input).render();
            pretty_assert_eq!(render, render2);
        }
    }
}
