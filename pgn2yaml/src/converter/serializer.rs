use crate::converter::interpreter::{
    CheckType, DetailedGame, DetailedMove, DetailedMoveInfo, GameEnding, MoveType, PromotionPieces,
};
use crate::reader::Tag;
use anyhow::{Context, Result};
use serde::ser::SerializeMap;
use serde::Serialize;
use std::path::PathBuf;

impl Serialize for DetailedMoveInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut entries = 2;
        if self.move_type != MoveType::Normal {
            entries += 1;
        }
        if self.check_type.is_some() {
            entries += 1;
        }

        let mut map = serializer.serialize_map(Some(entries))?;

        map.serialize_entry("short", &self.short)?;
        map.serialize_entry("long", &self.long)?;
        if self.move_type != MoveType::Normal {
            map.serialize_entry(
                "type",
                match &self.move_type {
                    MoveType::Normal => unreachable!(),
                    MoveType::Capture => "capture",
                    MoveType::PassingPawn => "passing pawn",
                    MoveType::EnPassant => "en passant",
                    MoveType::Promotion(piece) => match piece {
                        PromotionPieces::Knight => "promotion to knight",
                        PromotionPieces::Bishop => "promotion to bishop",
                        PromotionPieces::Rook => "promotion to rook",
                        PromotionPieces::Queen => "promotion to queen",
                    },
                    MoveType::PromotionWithCapture(piece) => match piece {
                        PromotionPieces::Knight => "promotion to knight with capture",
                        PromotionPieces::Bishop => "promotion to bishop with capture",
                        PromotionPieces::Rook => "promotion to rook with capture",
                        PromotionPieces::Queen => "promotion to queen with capture",
                    },
                    MoveType::KingsideCastling => "kingside castling",
                    MoveType::QueensideCastling => "queenside castling",
                },
            )?;
        }

        if let Some(check_type) = &self.check_type {
            map.serialize_entry(
                "check",
                match check_type {
                    CheckType::Check => "check",
                    CheckType::Checkmate => "checkmate",
                    CheckType::Stalemate => "stalemate",
                },
            )?;
        }

        map.end()
    }
}

impl Serialize for DetailedMove {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(black) = &self.black {
            let mut map = serializer.serialize_map(Some(2))?;

            map.serialize_entry("white", &self.white)?;
            map.serialize_entry("black", &black)?;

            map.end()
        } else {
            let mut map = serializer.serialize_map(Some(1))?;

            map.serialize_entry("white", &self.white)?;

            map.end()
        }
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.key, &self.value)?;
        map.end()
    }
}

struct SerializedMoveList<'a>(&'a Vec<DetailedMove>);

impl<'a> Serialize for SerializedMoveList<'a> {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (index, mv) in self.0.iter().enumerate() {
            let move_number = index + 1;
            map.serialize_entry(&move_number, mv)?;
        }

        map.end()
    }
}

struct SerializedGameEnding(GameEnding);

impl<'a> Serialize for SerializedGameEnding {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        let (result_str, reason_str) = match self.0 {
            GameEnding::Draw => ("1/2-1/2", "draw"),
            GameEnding::Stalemate => ("1/2-1/2", "stalemate"),
            GameEnding::WhiteWinsCheckmate => ("1-0", "checkmate"),
            GameEnding::BlackWinsCheckmate => ("0-1", "checkmate"),
            GameEnding::WhiteResigned => ("0-1", "resignation"),
            GameEnding::BlackResigned => ("1-0", "resignation"),
        };
        map.serialize_entry("result", result_str)?;
        map.serialize_entry("reason", reason_str)?;

        map.end()
    }
}

impl Serialize for DetailedGame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;

        map.serialize_entry("tags", &self.tags)?;

        map.serialize_entry("ending", &SerializedGameEnding(self.ending))?;
        map.serialize_entry("moves", &SerializedMoveList(&self.moves))?;

        map.end()
    }
}

pub fn write_yaml(yaml_path: &PathBuf, game: &DetailedGame) -> Result<()> {
    let output = std::fs::File::create(yaml_path).context(format!(
        "Could not open file {} for writing",
        yaml_path.to_string_lossy()
    ))?;

    serde_yaml::to_writer(output, game).context(format!(
        "Error writing YAML data to {}",
        yaml_path.to_string_lossy()
    ))
}
