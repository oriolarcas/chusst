mod interpreter;
mod serializer;

use self::interpreter::pgn_to_long_algebraic;
use self::serializer::write_yaml;
use crate::reader::PGN;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn write_pgn(pgn: &PGN, path: &PathBuf) -> Result<()> {
    let detailed_game =
        pgn_to_long_algebraic(&pgn).context("Cannot convert to long algebraic form")?;
    write_yaml(&path, &detailed_game)?;
    Ok(())
}
