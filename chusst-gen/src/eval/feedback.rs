use anyhow::Error;

use crate::game::{AddNode, GameTree, MoveAction, TreeNode};

#[derive(Clone)]
pub struct EngineFeedbackMessage {
    pub depth: u32, // in plies
    pub nodes: u32,
    pub score: i32, // in centipawns
}

pub struct EngineInfoMessage {
    pub message: String,
}

pub enum EngineMessage {
    SearchFeedback(EngineFeedbackMessage),
    Info(EngineInfoMessage),
}

pub trait EngineFeedback: std::io::Write {
    fn send(&self, msg: EngineMessage);
}

#[derive(Clone)]
pub struct SearchMove {
    pub mv: Option<MoveAction>,
    pub info: String,
}

#[derive(Clone)]
pub enum SearchNodeFeedback {
    Fen(String),
    Child(SearchMove),
    Info(String),
    Return,
}

pub trait SearchFeedback: std::io::Write {
    fn update(&mut self, depth: u32, nodes: u32, score: i32);
    fn info(&mut self, message: &str);
    fn search_node(&mut self, node_type: SearchNodeFeedback);
}

#[derive(Default)]
pub struct SilentSearchFeedback();

impl SearchFeedback for SilentSearchFeedback {
    fn update(&mut self, _depth: u32, _nodes: u32, _score: i32) {}

    fn info(&mut self, message: &str) {
        println!("{}", message);
    }

    fn search_node(&mut self, _node_type: SearchNodeFeedback) {}
}

impl std::io::Write for SilentSearchFeedback {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct PeriodicalSearchFeedback<'a> {
    update_interval: std::time::Duration,
    last_update: std::time::Instant,
    receiver: &'a mut dyn EngineFeedback,
}

impl<'a> PeriodicalSearchFeedback<'a> {
    pub fn new(
        update_interval: std::time::Duration,
        receiver: &'a mut impl EngineFeedback,
    ) -> Self {
        PeriodicalSearchFeedback {
            update_interval,
            last_update: std::time::Instant::now(),
            receiver,
        }
    }
}

impl<'a> SearchFeedback for PeriodicalSearchFeedback<'a> {
    fn update(&mut self, depth: u32, nodes: u32, score: i32) {
        let now = std::time::Instant::now();

        if now - self.last_update < self.update_interval {
            return;
        }

        self.receiver
            .send(EngineMessage::SearchFeedback(EngineFeedbackMessage {
                depth,
                nodes,
                score,
            }));

        self.last_update = now;
    }

    fn info(&mut self, message: &str) {
        self.receiver.send(EngineMessage::Info(EngineInfoMessage {
            message: message.to_string(),
        }))
    }

    fn search_node(&mut self, _node_type: SearchNodeFeedback) {}
}

impl<'a> std::io::Write for PeriodicalSearchFeedback<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.receiver.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.receiver.flush()
    }
}

pub struct StdoutFeedback;

impl EngineFeedback for StdoutFeedback {
    fn send(&self, _msg: EngineMessage) {}
}

impl SearchFeedback for StdoutFeedback {
    fn update(&mut self, _depth: u32, _nodes: u32, _score: i32) {}

    fn info(&mut self, message: &str) {
        println!("{}", message);
    }

    fn search_node(&mut self, _node_type: SearchNodeFeedback) {}
}

impl std::io::Write for StdoutFeedback {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        std::io::stdout().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

#[derive(Default)]
pub struct SearchTreeFeedback {
    nodes: Vec<SearchNodeFeedback>,
    log: bool,
}

fn into_tree<'a>(
    root: &mut impl AddNode,
    nodes: &mut impl Iterator<Item = &'a SearchNodeFeedback>,
) {
    while let Some(node_type) = nodes.next() {
        match node_type {
            SearchNodeFeedback::Fen(_fen) => {
                panic!("Cannot use a FEN position as a move in game tree")
            }
            SearchNodeFeedback::Child(child_move) => {
                if let Some(mv) = child_move.mv {
                    let mut child = TreeNode::new(mv);
                    child.add_comment(child_move.info.clone());
                    into_tree(&mut child, nodes);
                    root.add_node(child);
                } else {
                    root.add_comment(child_move.info.clone());
                }
            }
            SearchNodeFeedback::Info(info) => {
                root.add_comment(info.clone());
            }
            SearchNodeFeedback::Return => {
                return;
            }
        }
    }
}

impl TryFrom<SearchTreeFeedback> for GameTree {
    type Error = Error;

    fn try_from(value: SearchTreeFeedback) -> Result<Self, Self::Error> {
        let mut nodes_iter = value.nodes.iter();

        let first_node = nodes_iter.next();

        match first_node {
            Some(SearchNodeFeedback::Fen(fen)) => {
                let tree = GameTree::try_from_fen(fen);
                tree.map(|mut tree| {
                    into_tree(&mut tree, &mut nodes_iter);
                    tree
                })
                .ok_or(anyhow::anyhow!("Failed to parse FEN"))
            }
            _ => {
                let mut tree = GameTree::default();
                into_tree(&mut tree, &mut nodes_iter);
                Ok(tree)
            }
        }
    }
}

impl SearchTreeFeedback {
    pub fn with_logger() -> Self {
        Self {
            log: true,
            ..Default::default()
        }
    }
}

impl SearchFeedback for SearchTreeFeedback {
    fn update(&mut self, _depth: u32, _nodes: u32, _score: i32) {}

    fn info(&mut self, _message: &str) {}

    fn search_node(&mut self, node_type: SearchNodeFeedback) {
        self.nodes.push(node_type);
    }
}

impl std::io::Write for SearchTreeFeedback {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.log {
            std::io::stdout().write(buf)
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.log {
            std::io::stdout().flush()
        } else {
            Ok(())
        }
    }
}
