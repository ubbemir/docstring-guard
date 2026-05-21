use super::documentable::Documentable;
use crate::{utils, Language, MissingDocstring};

use anyhow::{Context, Result};
use rustpython_parser::{ast::Stmt, parse, text_size::TextRange, Mode};
use std::path::Path;

fn is_docstring(stmt: &Stmt) -> bool {
    stmt.as_expr_stmt()
        .and_then(|e| e.value.as_constant_expr())
        .is_some_and(|c| c.value.is_str())
}

fn is_dunder(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__")
}

fn get_line_number(content: &str, range: TextRange) -> usize {
    let start = range.start().to_usize();
    let slice = &content[..start];
    slice.chars().filter(|c| *c == '\n').count() + 1
}

fn check_statement_for_docstring(
    path: &Path,
    content: &str,
    stmt: &Stmt,
) -> Option<MissingDocstring> {
    match stmt {
        Stmt::FunctionDef(s) => check_documentable_for_docstring(path, content, s),
        Stmt::ClassDef(s) => check_documentable_for_docstring(path, content, s),
        _ => None,
    }
}

fn check_statements_for_docstrings(
    path: &Path,
    content: &str,
    stmts: &[Stmt],
) -> Vec<MissingDocstring> {
    let mut missing_docstrings = vec![];

    for stmt in stmts {
        if let Some(entry) = check_statement_for_docstring(path, content, stmt) {
            missing_docstrings.push(entry);
        }

        if let Some(s) = stmt.as_class_def_stmt() {
            missing_docstrings.extend(check_statements_for_docstrings(path, content, s.body()));
        }
    }

    missing_docstrings
}

fn check_documentable_for_docstring(
    path: &Path,
    content: &str,
    stmt: &impl Documentable,
) -> Option<MissingDocstring> {
    let range = stmt.range();
    let id = stmt.name().as_str();
    let line_number = get_line_number(content, range);

    if !is_dunder(id) && !utils::ignore_validation(Language::Python, line_number, content) {
        if let Some(docstring) = stmt.body().first() {
            if !is_docstring(docstring) {
                let entry = MissingDocstring {
                    file_name: path.display().to_string(),
                    name: id.to_string(),
                    line_number,
                };
                return Some(entry);
            }
        }
    }
    None
}

pub fn check_file_for_docstrings(file_path: impl AsRef<Path>) -> Result<Vec<MissingDocstring>> {
    let path = file_path.as_ref();
    let content =
        utils::load_file_content(path).with_context(|| utils::load_file_error_formating(path))?;
    let module: rustpython_parser::ast::Mod = parse(&content, Mode::Module, "<unknown>")
        .with_context(|| utils::parse_error_formating(path))?;

    let mut missing_docstrings: Vec<MissingDocstring> = vec![];
    if let Some(module) = &module.as_module() {
        missing_docstrings.extend(check_statements_for_docstrings(
            path,
            &content,
            &module.body,
        ));
    }
    Ok(missing_docstrings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("src/python/tests/fixtures/test_valid_docstring.py", vec![])]
    #[case("src/python/tests/fixtures/test_docstring_guard_ignore.py", vec![])]
    #[case("src/python/tests/fixtures/test_docstring_not_first.py",
        vec![
            MissingDocstring {
                file_name:"src/python/tests/fixtures/test_docstring_not_first.py".to_string(),
                name:"docstring_not_first".to_string(),
                line_number:3,
            }
        ]
    )]
    #[case("src/python/tests/fixtures/test_no_docstring_in_classdef.py",
        vec![
            MissingDocstring {
                    file_name:"src/python/tests/fixtures/test_no_docstring_in_classdef.py".to_string(),
                    name:"HelloWorld".to_string(),
                    line_number:3,
            },
            MissingDocstring {
                    file_name:"src/python/tests/fixtures/test_no_docstring_in_classdef.py".to_string(),
                    name:"no_docstring".to_string(),
                    line_number:6,
            }
        ]
    )]
    #[case("src/python/tests/fixtures/test_no_docstring_in_funcdef.py",
        vec![
            MissingDocstring {
                file_name:"src/python/tests/fixtures/test_no_docstring_in_funcdef.py".to_string(),
                name:"no_docstring".to_string(),
                line_number:3,
            }
        ]
    )]
    fn test_check_file_for_docstrings(
        #[case] input: impl AsRef<Path>,
        #[case] expected: Vec<MissingDocstring>,
    ) {
        assert_eq!(expected, check_file_for_docstrings(input).unwrap());
    }

    #[rstest]
    #[case("src/python/tests/fixtures/test_fail_to_read.py")]
    fn test_check_file_for_docstrings_read_errors(#[case] input: impl AsRef<Path>) {
        assert!(check_file_for_docstrings(input).is_err())
    }

    #[rstest]
    #[case("__init__", true)]
    #[case("test", false)]
    #[case("_test", false)]
    #[case("_test__", false)]
    #[case("__test_", false)]
    fn test_is_dunder(#[case] input: String, #[case] expected: bool) {
        assert_eq!(expected, is_dunder(&input))
    }

    #[rstest]
    #[case(
        "def hello_world():\n\t\"\"\"prints 'Hello World'\"\"\"\n\tprint(\"Hello World\")",
        true
    )]
    #[case("def hello_world():\n\tprint(\"Hello World\")", false)]
    fn test_is_docstring_true(#[case] input: String, #[case] expected: bool) {
        let ast = parse(&input, Mode::Module, "unknown").unwrap();
        let stmt = ast.as_module().unwrap().body[0]
            .as_function_def_stmt()
            .unwrap();
        assert_eq!(expected, is_docstring(&stmt.body()[0]));
    }

    #[rstest]
    #[case(
        "def hello_world(): #docstring-guard=ignore\n\t\n\tprint(\"Hello World\")",
        1,
        true
    )]
    #[case("def hello_world():\n\t\n\tprint(\"Hello World\")", 1, false)]
    fn test_ignore_validation(
        #[case] input: String,
        #[case] line_number: usize,
        #[case] expected: bool,
    ) {
        assert_eq!(
            expected,
            utils::ignore_validation(Language::Python, line_number, &input)
        );
    }
}
