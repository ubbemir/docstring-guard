pub mod python;
pub mod rust;
pub use python::checker::check_file_for_docstrings as check_python_file;
pub use rust::checker::check_file_for_docstrings as check_rust_file;
pub mod utils;

#[derive(Clone, Copy)]
pub enum Language {
    Python,
    Rust,
}

#[derive(PartialEq, Debug)]
pub struct MissingDocstring {
    pub file_name: String,
    pub name: String,
    pub line_number: usize,
}
