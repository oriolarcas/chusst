mod converter;
mod reader;

use anyhow::{bail, Context, Result};
use clap::Parser;
use crate::converter::write_pgn;
use crate::reader::parse_pgn_file;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// PGN file
    file: String,

    /// Path of the YAML file (if not specified, same as the PGN file with the extension changed)
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let pgn_path = PathBuf::from(cli.file);

    let pgns = parse_pgn_file(&pgn_path).context(format!(
        "Unable to parse PGN file {}",
        pgn_path.to_string_lossy()
    ))?;

    let yaml_path = cli.output.map_or(
        {
            let mut path = pgn_path;
            path.set_extension("yaml");
            println!("Writing YAML file to {}", path.to_string_lossy());
            path
        },
        PathBuf::from,
    );

    match pgns.len() {
        0 => bail!("No games found in PGN file"),
        1 => write_pgn(&pgns[0], &yaml_path)?,
        _ => {
            for (index, pgn) in pgns.iter().enumerate() {
                let mut path = yaml_path.clone();
                path.set_file_name(format!(
                    "{}.{}.{}",
                    path.file_stem().unwrap().to_string_lossy(),
                    index,
                    path.extension().unwrap().to_string_lossy()
                ));
                write_pgn(pgn, &path)?;
            }
        }
    }

    Ok(())
}
