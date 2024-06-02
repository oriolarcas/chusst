use anyhow::Error;
use serde::Serialize;

use crate::eval::Game;

use super::{MoveAction, SimpleGame};

pub trait AddNode {
    fn add_node(&mut self, child: TreeNode);
    fn add_comment(&mut self, comment: String);
}

pub struct TreeNode {
    mv: MoveAction,
    mv_name: Option<String>,
    comments: Vec<String>,
    nodes: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(mv: MoveAction) -> TreeNode {
        TreeNode {
            mv,
            mv_name: None,
            comments: Vec::new(),
            nodes: Vec::new(),
        }
    }
}

impl AddNode for TreeNode {
    fn add_node(&mut self, child: TreeNode) {
        self.nodes.push(child);
    }

    fn add_comment(&mut self, comment: String) {
        self.comments.push(comment);
    }
}

pub struct GameTree {
    initial_position: SimpleGame,
    nodes: Vec<TreeNode>,
}

impl GameTree {
    pub fn try_from_fen(fen: &str) -> Option<GameTree> {
        Some(GameTree {
            initial_position: SimpleGame::try_from_fen(
                &fen.split_ascii_whitespace().collect::<Vec<_>>(),
            )?,
            nodes: Vec::new(),
        })
    }
}

impl Default for GameTree {
    fn default() -> GameTree {
        GameTree {
            initial_position: SimpleGame::new(),
            nodes: Vec::new(),
        }
    }
}

impl AddNode for GameTree {
    fn add_node(&mut self, child: TreeNode) {
        self.nodes.push(child);
    }

    fn add_comment(&mut self, _comment: String) {
        // Do nothing
    }
}

impl Serialize for TreeNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let move_name = self.mv_name.clone().unwrap_or(format!("{}", self.mv.mv));

        if self.nodes.is_empty() {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("move", &move_name)?;
            map.serialize_entry("comments", &self.comments)?;
            map.end()
        } else {
            let mut map = serializer.serialize_map(Some(3))?;
            map.serialize_entry("move", &move_name)?;
            map.serialize_entry("comments", &self.comments)?;
            map.serialize_entry("nodes", &self.nodes)?;
            map.end()
        }
    }
}

impl Serialize for GameTree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(2))?;

        map.serialize_entry("initial_position", &self.initial_position.to_fen())?;

        map.serialize_entry("nodes", &self.nodes)?;

        map.end()
    }
}

fn add_move_names(game: SimpleGame, node: &mut TreeNode) -> Result<(), Error> {
    let mv = node.mv;

    let mv_name = game.move_name(&mv)?;
    node.mv_name = Some(mv_name);

    let mut game = game;
    game.do_move(&mv)
        .ok_or(anyhow::anyhow!("Failed to do move"))?;

    for node in &mut node.nodes {
        add_move_names(game.clone(), node)?;
    }

    Ok(())
}

impl GameTree {
    pub fn try_json(&mut self) -> Result<String, Error> {
        self.add_move_names()?;
        serde_json::to_string_pretty(&self).map_err(|err| err.into())
    }

    fn add_move_names(&mut self) -> Result<(), Error> {
        let game = self.initial_position.clone();
        for node in &mut self.nodes {
            add_move_names(game.clone(), node)?;
        }
        Ok(())
    }
}
