use indoc::indoc;
use regex::Regex;

use lazy_static::lazy_static;

/// Returns a regex that matches markdown files if they contain the given tag
///
/// # Arguments
///
/// * `tag` - The tag to match
pub fn markdown_contains_tag(tag: &str) -> Result<Regex, regex::Error> {
    Regex::new(
        format!(
            r"(?sx)^
        (?:                 # match the optional frontmatter section
            \n*                 # leading newlines
            \-{{3}}             # frontmatter starts with `---`
            .*\n                # frontmatter content  
            \-{{3}}\n           # frontmatter ends with `---\n`
        )?
        \n*                 # match leading newlines
        (?:\#[\w\-/]+\s)*   # match other tags
        \#{tag}             # match the arbitrary tag"
        )
        .as_str(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    lazy_static! {
        static ref IS_TAGGED_TODO: Regex = markdown_contains_tag("todo").unwrap();
    }

    macro_rules! markdown_contains_tag_tests {
      ($($name:ident: $document:expr)*) => {
      $(
          #[test]
          fn $name() {
              let document = indoc!($document);
              let is_tagged = stringify!($name).starts_with("tagged");
              assert_eq!(is_tagged, IS_TAGGED_TODO.is_match(document));
          }
      )*
      }
    }

    markdown_contains_tag_tests! {
      untagged_document: r#"
        - [ ] test
    "#
      tagged_document: r#"
        #todo #other
        - [ ] test
    "#
      tagged_document_with_frontmatter: r#"
        ---
        title: test
        ---
        #todo #other
        - [ ] test
    "#
      tagged_document_with_frontmatter_and_newlines: r#"
        ---
        title: test
        ---

        #other
        #todo

        - [ ] test
    "#
      untagged_document_with_tag_in_content: r#"
        - [ ] #todo test
    "#
      untagged_document_with_tag_in_frontmatter: r#"
        ---
        title: #todo test
        ---
        - [ ] test
    "#
      tagged_document_with_subtag: r#"
        #todo/subtag
        - [ ] test
    "#
    }
}
