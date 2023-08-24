use std::{borrow::Cow, cell::Cell, mem, path::PathBuf};

use lazy_static::lazy_static;
use markdown::mdast::{self, Node};
use rayon::prelude::ParallelIterator;
use regex::Regex;

use crate::{
    markdown_file::{File, MdastDocument},
    util::iterate_markdown_files,
};

lazy_static! {
    static ref TEXT_REPLACE: Regex = Regex::new("(--)").unwrap();
}

fn text_replace(text: String) -> String {
    match TEXT_REPLACE.replace_all(&text, "—") {
        Cow::Borrowed(_) => text,
        Cow::Owned(text) => text,
    }
}

fn format_node(mut node: Node, changed: &Cell<bool>) -> Node {
    match node {
        Node::Text(text) => Node::Text(mdast::Text {
            value: text_replace(text.value),
            position: text.position,
        }),
        _ => {
            if let Some(children) = node.children_mut() {
                for i in 0..children.len() {
                    let owned_child = mem::replace(
                        &mut children[i],
                        Node::Text(mdast::Text {
                            value: String::new(),
                            position: Default::default(),
                        }),
                    );
                    let new_child = format_node(owned_child, changed);
                    children[i] = new_child;
                }
            }
            node
        }
    }
}

fn format_document(document: MdastDocument) -> Option<MdastDocument> {
    let changed = Cell::new(false);
    let new_body = match format_node(Node::Root(document.body), &changed) {
        Node::Root(root) => root,
        _ => unreachable!(),
    };

    Some(MdastDocument {
        body: new_body,
        frontmatter: document.frontmatter,
    })
}

pub fn format_files(vault_path: PathBuf) {
    iterate_markdown_files(vault_path)
        .map(|file| (file.path, MdastDocument::parse(file.content.as_str())))
        .filter_map(|(path, document)| {
            format_document(document).map(|document| (path, document.render()))
        })
        .map(|(path, content)| {
            println!("Formatting {}", path.display());
            File::atomic_overwrite(&path, content)
        })
        .for_each(Result::unwrap);
}
