use std::fmt;

use console::Style;
use similar::{ChangeTag, TextDiff};

struct Line(Option<usize>);

// lifted from https://github.com/mitsuhiko/similar/blob/de455873dab514082bf6e7bb5f0029837fe280d5/examples/terminal-inline.rs

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

#[must_use]
pub fn diff(mut stdout_buffer: Vec<String>, old: &str, new: &str) -> Vec<String> {
    let diff = TextDiff::from_lines(old, new);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            stdout_buffer.push(format!("{:-^1$}\n", "-", 80));
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                stdout_buffer.push(format!(
                    "{}{} |{}",
                    s.apply_to(Line(change.old_index())).dim(),
                    s.apply_to(Line(change.new_index())).dim(),
                    s.apply_to(sign).bold(),
                ));
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        stdout_buffer
                            .push(format!("{}", s.apply_to(value).underlined().on_black()));
                    } else {
                        stdout_buffer.push(format!("{}", s.apply_to(value)));
                    }
                }
                if change.missing_newline() {
                    stdout_buffer.push(format!("\n"));
                }
            }
        }
    }

    stdout_buffer
}
