use chusst::board::{Game, Move};
use chusst::moves::{do_move, get_best_move_with_logger, GameMove, HasStopSignal};

use std::fs::File;
use std::io::Write;
use std::time::Instant;

#[derive(Clone)]
struct GoCommand {
    depth: u32,
}

#[derive(Clone)]
struct NewGameCommand {
    game: Option<Game>,
    moves: Vec<Move>,
}

#[derive(Clone)]
enum EngineCommand {
    NewGame(NewGameCommand),
    Go(GoCommand),
    Stop,
    Exit,
}

enum EngineResponse {
    Ready,
    Log(String),
    BestBranch(Option<GameMove>),
    Error(String),
}

struct Logger {
    file: File,
    last_event: std::time::Instant,
}

impl Logger {
    pub fn update(&mut self) -> f64 {
        let now = Instant::now();
        let elapsed = self.last_event.elapsed();
        self.last_event = now;
        (elapsed.as_micros() as f64) / 1000.
    }
}

struct SenderAsWriter<'a> {
    sender: &'a mut std::sync::mpsc::Sender<EngineResponse>,
}

impl<'a> std::io::Write for SenderAsWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8(buf.to_vec())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        let msg_len = msg.len();
        self.sender
            .send(EngineResponse::Log(msg))
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        Ok(msg_len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

const SEARCH_DEPTH_DEFAULT: u32 = 4;
const SEARCH_DEPTH_MIN: u32 = 2;
const SEARCH_DEPTH_MAX: u32 = 5;

pub fn uci_loop() {
    let mut logger = Logger {
        file: match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("uci.log")
        {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Could not open the log file: {}", err);
                return;
            }
        },
        last_event: Instant::now(),
    };

    macro_rules! write_command {
        ($str:expr) => {
            {
                let elapsed = logger.update();
                let _ = writeln!(logger.file, "< +{} {}", elapsed, $str);
                println!($str);
            }
        };
        ($fmt:expr, $($param:expr),*) => {
            {
                let elapsed = logger.update();
                let _ = writeln!(logger.file, "< +{} {}", elapsed, format!($fmt, $($param),+));
                println!($fmt, $($param),+);
            }
        };
    }

    macro_rules! log {
        ($str:expr) => {
            {
                let _ = writeln!(logger.file, $str);
            }
        };
        ($fmt:expr, $($param:expr),*) => {
            {
                let _ = writeln!(logger.file, $fmt, $($param),+);
            }
        };
    }

    let mut search_depth = 3;
    let (engine_thread, to_engine, from_engine) = {
        // Communication channels
        let (to_engine_send, to_engine_receive) = std::sync::mpsc::channel::<EngineCommand>();
        let (from_engine_send, from_engine_receive) = std::sync::mpsc::channel::<EngineResponse>();

        // Spawn thread
        let engine_thread =
            std::thread::spawn(move || engine_thread(to_engine_receive, from_engine_send));

        (engine_thread, to_engine_send, from_engine_receive)
    };

    log!("Starting engine");

    match from_engine.recv() {
        Ok(EngineResponse::Ready) => (),
        _ => {
            log!("Could not start engine thread");
            return;
        }
    }

    log!("Engine ready");

    loop {
        let words = read_command(&mut logger);

        let command = match words.first() {
            Some(word) => word.to_owned(),
            None => {
                log!("Empty input");
                continue;
            }
        };

        let params = words
            .get(1..)
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>();

        match command.as_str() {
            "uci" => {
                write_command!("id name Chusst {}", env!("CARGO_PKG_VERSION"));
                write_command!(
                    "option name SearchDepth type spin default {} min {} max {}",
                    SEARCH_DEPTH_DEFAULT,
                    SEARCH_DEPTH_MIN,
                    SEARCH_DEPTH_MAX
                );
                write_command!("uciok");
            }
            "isready" => write_command!("readyok"),
            "setparam" => {
                if let Some(&["name", name, "value", value]) = params.get(0..4) {
                    match name {
                        "SearchDepth" => {
                            if let Ok(int_value) = value.parse::<u32>() {
                                match int_value {
                                    SEARCH_DEPTH_MIN..=SEARCH_DEPTH_MAX => search_depth = int_value,
                                    _ => log!("SearchDepth value out of range"),
                                }
                            } else {
                                log!("Invalid SearchDepth value");
                            }
                        }
                        _ => log!("Unknown parameter"),
                    }
                } else {
                    log!("Malformed setparam command");
                }
            }
            "ucinewgame" => {
                if let Err(_) = to_engine.send(EngineCommand::NewGame(NewGameCommand {
                    game: Some(Game::new()),
                    moves: Vec::new(),
                })) {
                    log!("Error: could not send new game to engine");
                    break;
                }
            }
            "position" => {
                let mut param_iter = params.iter();
                let (next_token, new_game) = match param_iter.next().map(|token| *token) {
                    Some("startpos") => (param_iter.next().map(|token| *token), Some(Game::new())),
                    Some("fenstring") => {
                        let fen = param_iter
                            .clone()
                            .take(6)
                            .map(|param| *param)
                            .collect::<Vec<&str>>();
                        if let Some(new_game_from_fen) = Game::try_from_fen(fen.as_slice()) {
                            (
                                param_iter.next().map(|token| *token),
                                Some(new_game_from_fen),
                            )
                        } else {
                            log!("Malformed FEN string in position command");
                            continue;
                        }
                    }
                    Some(token) => (Some(token), None),
                    _ => {
                        log!("Malformed position command");
                        continue;
                    }
                };
                let mut new_game_command = NewGameCommand {
                    game: new_game,
                    moves: Vec::new(),
                };
                if let Some("moves") = next_token {
                    for mv_str in param_iter {
                        if let Some(mv) = Move::try_from_long_algebraic_str(&mv_str) {
                            new_game_command.moves.push(mv);
                        } else {
                            log!("Malformed move in position command");
                            continue;
                        }
                    }
                }
                if to_engine
                    .send(EngineCommand::NewGame(new_game_command))
                    .is_err()
                {
                    log!("Error: could not send new game command to engine");
                    break;
                }
            }
            "go" => match params.first() {
                Some(&"infinite") => {
                    if to_engine
                        .send(EngineCommand::Go(GoCommand {
                            depth: search_depth,
                        }))
                        .is_err()
                    {
                        log!("Error: could not send go command to engine");
                        break;
                    }
                }
                _ => log!("Unknown go command"),
            },
            "stop" => {
                if let Err(_) = to_engine.send(EngineCommand::Stop) {
                    log!("Error: could not send go command to engine");
                    break;
                }
                match from_engine.try_recv() {
                    Ok(EngineResponse::BestBranch(best_move_result)) => {
                        let best_move_str = match best_move_result {
                            Some(GameMove::Normal(best_move)) => {
                                format!("{}{}", best_move.source, best_move.target)
                            }
                            Some(GameMove::Mate(_)) | None => "0000".to_owned(),
                        };
                        write_command!("bestmove {}", best_move_str);
                    }
                    Ok(EngineResponse::Log(msg)) => log!("{}", msg),
                    Ok(EngineResponse::Ready) => {
                        log!("Unexpected Ready command from Engine");
                        break;
                    }
                    Ok(EngineResponse::Error(reason)) => {
                        log!("Engine error: {}", reason);
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => (),
                    Err(_) => {
                        log!("Error communicating with the engine");
                        break;
                    }
                }
            }
            "quit" => {
                break;
            }
            _ => log!("Unknown command"),
        }
    }

    let _ = to_engine.send(EngineCommand::Exit);
    let _ = engine_thread.join();
}

fn read_command(logger: &mut Logger) -> Vec<String> {
    let mut buffer = String::new();
    if let Err(_) = std::io::stdin().read_line(&mut buffer) {
        return vec![];
    }

    let trimmed_buffer = buffer.trim();
    let elapsed = logger.update();
    let _result = writeln!(logger.file, "> +{} {}", elapsed, trimmed_buffer);
    trimmed_buffer
        .split_whitespace()
        .map(String::from)
        .collect()
}

struct EngineCommandReceiver<'a> {
    receiver: &'a std::sync::mpsc::Receiver<EngineCommand>,
    messages: Vec<EngineCommand>,
}

impl<'a> HasStopSignal for EngineCommandReceiver<'a> {
    fn stop(&mut self) -> bool {
        if let Ok(cmd) = self.receiver.try_recv() {
            match cmd {
                EngineCommand::Stop => return true,
                _ => self.messages.push(cmd),
            }
        }
        false
    }
}

fn engine_thread(
    to_engine: std::sync::mpsc::Receiver<EngineCommand>,
    from_engine: std::sync::mpsc::Sender<EngineResponse>,
) {
    let mut from_engine_mut = from_engine;
    let mut communicator = SenderAsWriter {
        sender: &mut from_engine_mut,
    };
    let mut game = Game::new();
    let mut command_receiver = EngineCommandReceiver {
        receiver: &to_engine,
        messages: Vec::new(),
    };

    if communicator.sender.send(EngineResponse::Ready).is_err() {
        return;
    }

    loop {
        let command = command_receiver.messages.pop().or(to_engine.recv().ok());
        match command {
            Some(EngineCommand::NewGame(new_game_cmd)) => {
                if let Some(new_game) = new_game_cmd.game {
                    game = new_game;
                }
                for mv in new_game_cmd.moves {
                    if do_move(&mut game, &mv).is_none() {
                        let _ = communicator
                            .sender
                            .send(EngineResponse::Error(format!("Invalid move {}", mv)));
                    }
                }
            }
            Some(EngineCommand::Go(go_command)) => {
                let best_move = get_best_move_with_logger(
                    &mut game,
                    go_command.depth,
                    &mut command_receiver,
                    &mut communicator,
                );
                let _ignore_error = communicator
                    .sender
                    .send(EngineResponse::BestBranch(Some(best_move)));
            }
            Some(EngineCommand::Stop) => (),
            Some(EngineCommand::Exit) => break,
            None => break,
        }
    }
}

fn main() {
    uci_loop();
}
