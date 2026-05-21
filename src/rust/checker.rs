use super::documentable::Documentable;
use crate::{utils, Language, MissingDocstring};

use anyhow::{Context, Result};
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{ItemFn, ItemStruct, Visibility};

struct DocstringVisitor<'a> {
    file_name: String,
    missing_docstrings: Vec<MissingDocstring>,
    file_content: &'a str,
}

impl<'ast> Visit<'ast> for DocstringVisitor<'_> {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if !utils::ignore_validation(
            Language::Rust,
            i.sig.ident.span().start().line,
            self.file_content,
        ) {
            if let Some(missing_docstring) = has_docstring(&self.file_name, i) {
                self.missing_docstrings.push(missing_docstring);
            }
        }
        visit::visit_item_fn(self, i);
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        if !utils::ignore_validation(
            Language::Rust,
            i.ident.span().start().line,
            self.file_content,
        ) {
            if let Some(missing_docstring) = has_docstring(&self.file_name, i) {
                self.missing_docstrings.push(missing_docstring);
            }
        }
        visit::visit_item_struct(self, i);
    }
}

fn has_docstring(file_name: &str, item: &impl Documentable) -> Option<MissingDocstring> {
    match item.vis() {
        Visibility::Public(_) => {
            if !item.attrs().iter().any(|attr| attr.path().is_ident("doc")) {
                let entry = MissingDocstring {
                    file_name: file_name.to_owned(),
                    name: item.ident().to_string(),
                    line_number: item.ident().span().start().line,
                };
                return Some(entry);
            }
            None
        }
        _ => None,
    }
}

pub fn check_file_for_docstrings(file_path: impl AsRef<Path>) -> Result<Vec<MissingDocstring>> {
    let path = file_path.as_ref();
    let content =
        utils::load_file_content(path).with_context(|| utils::load_file_error_formating(path))?;

    let ast = syn::parse_file(&content).with_context(|| utils::parse_error_formating(path))?;

    let mut visitor = DocstringVisitor {
        file_name: path.display().to_string(),
        missing_docstrings: Vec::new(),
        file_content: &content,
    };
    visitor.visit_file(&ast);

    Ok(visitor.missing_docstrings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("src/rust/tests/fixtures/test_documented_fn.rs",
        vec![]
    )]
    #[case("src/rust/tests/fixtures/test_documented_struct.rs",
        vec![]
    )]
    #[case("src/rust/tests/fixtures/test_private_fn_skip.rs",
        vec![]
    )]
    #[case("src/rust/tests/fixtures/test_private_struct_skip.rs",
        vec![]
    )]
    #[case("src/rust/tests/fixtures/test_undocumented_fn.rs",
        vec![
            MissingDocstring {
                file_name:"src/rust/tests/fixtures/test_undocumented_fn.rs".to_string(),
                name:"undocumented_function".to_string(),
                line_number:1,
            }
        ]
    )]
    #[case("src/rust/tests/fixtures/test_undocumented_struct.rs",
        vec![
            MissingDocstring {
                file_name:"src/rust/tests/fixtures/test_undocumented_struct.rs".to_string(),
                name:"UndocumentedStruct".to_string(),
                line_number:1,
            }
        ]
    )]
    fn test_check_file_for_docstrings(
        #[case] input: impl AsRef<Path>,
        #[case] expected: Vec<MissingDocstring>,
    ) {
        assert_eq!(expected, check_file_for_docstrings(input).unwrap());
    }
}
