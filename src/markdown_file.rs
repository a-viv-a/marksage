use std::{convert::identity, fmt::format, fs, io, path::PathBuf};

use markdown::mdast::{self, Node};

pub struct File {
    path: PathBuf,
    pub content: String,
}

impl File {
    pub fn at_path(path: PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(&path)?;
        Ok(Self { path, content })
    }

    pub fn atomic_overwrite(self, content: &str) -> io::Result<()> {
        let mut tmp_path = self.path.clone();
        tmp_path.set_extension("tmp.md");
        fs::write(&tmp_path, content)?;
        fs::rename(tmp_path, self.path)?;
        Ok(())
    }
}

fn ast_string(nodes: &Vec<Node>) -> String {
    nodes
        .iter()
        .map(mdast_to_markdown)
        .collect::<Vec<String>>()
        .join("")
}

fn indent(li: &mdast::ListItem) -> String {
    li.position
        .as_ref()
        .map(|p| " ".repeat((p.start.column - 1) * 2))
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

pub fn mdast_to_markdown(node: &Node) -> String {
    match node {
        Node::Root(_) => ast_string(node.children().unwrap()),
        Node::Heading(heading) => {
            format!(
                "{} {}\n\n",
                "#".repeat(heading.depth as usize),
                ast_string(node.children().unwrap())
            )
        }
        Node::Text(t) => t.value.clone(),
        Node::Paragraph(p) => format!("{}\n", ast_string(&p.children)),
        Node::List(l) => ast_string(&l.children),
        Node::ListItem(li) => format!(
            "{}- {}{}",
            indent(li),
            match li.checked {
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            },
            ast_string(&li.children)
        ),
        Node::Code(c) => format!(
            "```{}\n{}\n```",
            c.lang.as_ref().unwrap_or(&String::new()),
            c.value
        ),
        Node::InlineCode(c) => {
            let backtick = "`".repeat(count_longest_sequential_chars(&c.value, '`') + 1);
            format!("{}{}{}", backtick, c.value, backtick)
        }
        _ => panic!("Unexpected node type {:#?}", node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    macro_rules! test_mdast_to_markdown {
        ($($name:ident $file:expr)*) => {
            $(
                #[test]
                fn $name() {
                    let file = indoc!($file);
                    let ast = markdown::to_mdast(file, &markdown::ParseOptions::gfm()).expect("never fails with gfm");
                    let render = mdast_to_markdown(&ast);
                    assert_eq!(file, &render, "input file (left) did not match rendered markdown (right). ast:\n{:#?}\n\ntest: {}\nexpected:\n{}\nactual:\n{}", ast, stringify!($name), file, render);
                }
            )*
        }
    }

    test_mdast_to_markdown! {
        simple_file r#"
        # Heading

        - [ ] item 1
        - [x] item 2
        "#

        nested_list r#"
        - [ ] item 1
            - [ ] item 1.1
            - [x] item 1.2
        - [x] item 2
        "#

        multiple_headers r#"
        # Heading 1

        - [ ] item 1

        ## Heading 2

        some text
        "#

        code_block r#"
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

        lists r#"
        # Heading

        - item 1
        - item 2
        "#
    }
}
