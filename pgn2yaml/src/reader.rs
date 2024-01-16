mod lexer;
mod parser;

pub use self::parser::{Tag, Pgn};

use anyhow::Result;
use parser::Parser;
use std::path::PathBuf;

pub fn parse_pgn_file(pgn_file_path: &PathBuf) -> Result<Vec<Pgn>> {
    Parser::parse_file(pgn_file_path)
}
