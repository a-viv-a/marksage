use regex::Regex;
use walkdir::DirEntry;

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

pub fn is_visible(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref IS_TAGGED_TODO: Regex = markdown_contains_tag("todo").unwrap();
    }

    macro_rules! markdown_contains_tag_tests {
      ($($name:ident $document:expr)*) => {
      $(
          #[test]
          fn $name() {
              // error if name doesn't start with tagged or untagged
              assert!(stringify!($name).starts_with("tagged") || stringify!($name).starts_with("untagged"));

              let document = indoc!($document);
              let is_tagged = stringify!($name).starts_with("tagged");
              assert_eq!(is_tagged, IS_TAGGED_TODO.is_match(document));
          }
      )*
      }
    }

    markdown_contains_tag_tests! {
      untagged_document r#"
        - [ ] test
    "#
      tagged_document r#"
        #todo #other
        - [ ] test
    "#
      tagged_document_with_frontmatter r#"
        ---
        title: test
        ---
        #todo #other
        - [ ] test
    "#
      tagged_document_with_frontmatter_and_newlines r#"
        ---
        title: test
        ---

        #other
        #todo

        - [ ] test
    "#
      untagged_document_with_tag_in_content r#"
        - [ ] #todo test
    "#
      untagged_document_with_tag_in_frontmatter r#"
        ---
        title: #todo test
        ---
        - [ ] test
    "#
      tagged_document_with_sub_tag r#"
        #todo/sub-tag
        - [ ] test
    "#
      untagged_document_with_tag_after_header r#"
        # Header
        
        #todo some stuff
      "#
    }
}
