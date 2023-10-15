mod duplex_thread;
mod engine;
mod stdin;

use engine::{EngineCommand, EngineResponse, GoCommand, NewGameCommand};
use chusst::board::{Game, Move};
use chusst::moves::GameMove;

use crossbeam_channel::select;
use rust_fsm::*;

use std::fs::File;
use std::io::Write;
use std::time::Instant;

enum SyncInput {
    FromStdin(String),
    FromEngine(EngineResponse),
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

state_machine! {
    // derive(Debug)
    UciProtocol(Initializing)

    Initializing(CommandUci) => Ready[OutputCommandId],
    Ready => {
        CommandGo => Searching[EngineCommandGo],
        CommandPosition => Ready[EngineCommandPosition],
        // Additional commands
        CommandUciNewGame => Ready[EngineCommandNewGame],
        CommandIsReady => Ready[OutputCommandReadyOk],
        CommandSetParam => Ready[EngineCommandSetParam],
        CommandStop => Ready,
    },
    Searching => {
        CommandStop => WaitingForResult[EngineCommandStop],
        // Additional commands
        CommandIsReady => Ready[OutputCommandReadyOk],
    },
    WaitingForResult => {
        EngineResult => Ready[OutputCommandBestMove],
        // Additional commands
        CommandStop => WaitingForResult,
    }
}

enum ParsedInput {
    UciStdInInput(Vec<String>),
    EngineMessage(EngineResponse),
}

const SEARCH_DEPTH_DEFAULT: u32 = 4;
const SEARCH_DEPTH_MIN: u32 = 2;
const SEARCH_DEPTH_MAX: u32 = 5;

fn uci_loop<'scope, 'env>(scope: &'scope std::thread::Scope<'scope, 'env>) {
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
    let engine_thread = engine::create_engine_thread(scope);
    let stdin_thread = stdin::create_stdin_thread(scope);

    log!("Starting engine");

    match engine_thread.from_thread.recv() {
        Ok(EngineResponse::Ready) => (),
        _ => {
            log!("Could not start engine thread");
            return;
        }
    }

    log!("Engine ready");

    let mut uci_protocol: StateMachine<UciProtocol> = StateMachine::new();

    let get_input = || -> Option<SyncInput> {
        select! {
            recv(stdin_thread.from_thread) -> stdin_line => return Some(SyncInput::FromStdin(stdin_line.ok()?)),
            recv(engine_thread.from_thread) -> engine_response => return Some(SyncInput::FromEngine(engine_response.ok()?)),
        }
    };

    loop {
        let input = get_input();

        let (protocol_input, parsed_input) = match &input {
            Some(SyncInput::FromStdin(stdin_line)) => {
                let words = parse_command(stdin_line.as_str(), &mut logger);
                let protocol_stdin_input = if let Some(stdin_command) = words.first() {
                    match stdin_command.as_str() {
                        "uci" => UciProtocolInput::CommandUci,
                        "isready" => UciProtocolInput::CommandIsReady,
                        "setparam" => UciProtocolInput::CommandSetParam,
                        "ucinewgame" => UciProtocolInput::CommandUciNewGame,
                        "position" => UciProtocolInput::CommandPosition,
                        "go" => UciProtocolInput::CommandGo,
                        "stop" => UciProtocolInput::CommandStop,
                        // Exit
                        "quit" => break,
                        // Unknown command, ignore:
                        _ => {
                            log!("Unknown command");
                            continue;
                        }
                    }
                } else {
                    continue;
                };
                (protocol_stdin_input, ParsedInput::UciStdInInput(words))
            }
            Some(SyncInput::FromEngine(engine_response)) => {
                let engine_protocol_input = match &engine_response {
                    EngineResponse::Log(message) => {
                        log!("{}", message);
                        continue;
                    }
                    EngineResponse::Ready => continue,
                    EngineResponse::BestBranch(_) => UciProtocolInput::EngineResult,
                    EngineResponse::Error(error) => {
                        log!("{}", error);
                        break;
                    }
                };
                (
                    engine_protocol_input,
                    ParsedInput::EngineMessage(engine_response.clone()),
                )
            }
            None => {
                log!("I/O error, exiting");
                break;
            }
        };

        let protocol_output_result = uci_protocol.consume(&protocol_input);

        let protocol_output = match &protocol_output_result {
            Ok(output) => output,
            Err(_) => {
                log!("Unexpected UCI command, ignoring");
                continue;
            }
        };

        match (protocol_output, parsed_input) {
            (Some(UciProtocolOutput::OutputCommandId), _) => {
                write_command!("id name Chusst {}", env!("CARGO_PKG_VERSION"));
                write_command!(
                    "option name SearchDepth type spin default {} min {} max {}",
                    SEARCH_DEPTH_DEFAULT,
                    SEARCH_DEPTH_MIN,
                    SEARCH_DEPTH_MAX
                );
                write_command!("uciok");
            }
            (Some(UciProtocolOutput::EngineCommandSetParam), ParsedInput::UciStdInInput(words)) => {
                if let Some(&["name", name, "value", value]) = words
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<&str>>()
                    .get(1..5)
                {
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
            (Some(UciProtocolOutput::OutputCommandReadyOk), ParsedInput::UciStdInInput(_)) => {
                write_command!("readyok")
            }
            (Some(UciProtocolOutput::EngineCommandNewGame), ParsedInput::UciStdInInput(_)) => {
                if let Err(_) =
                    engine_thread
                        .to_thread
                        .send(EngineCommand::NewGame(NewGameCommand {
                            game: Some(Game::new()),
                            moves: Vec::new(),
                        }))
                {
                    log!("Error: could not send new game to engine");
                    break;
                }
            }
            (Some(UciProtocolOutput::EngineCommandPosition), ParsedInput::UciStdInInput(words)) => {
                let mut param_iter = words.iter().skip(1).map(String::as_str);
                let (next_token, new_game) = match param_iter.next() {
                    Some("startpos") => (param_iter.next(), Some(Game::new())),
                    Some("fenstring") => {
                        let fen = param_iter.clone().take(6).collect::<Vec<&str>>();
                        if let Some(new_game_from_fen) = Game::try_from_fen(fen.as_slice()) {
                            (param_iter.next(), Some(new_game_from_fen))
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
                if engine_thread
                    .to_thread
                    .send(EngineCommand::NewGame(new_game_command))
                    .is_err()
                {
                    log!("Error: could not send new game command to engine");
                    break;
                }
            }
            (Some(UciProtocolOutput::EngineCommandGo), ParsedInput::UciStdInInput(words)) => {
                match words.get(1).map(String::as_str) {
                    Some("infinite") => {
                        if engine_thread
                            .to_thread
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
                }
            }
            (Some(UciProtocolOutput::EngineCommandStop), ParsedInput::UciStdInInput(_)) => {
                if let Err(_) = engine_thread.to_thread.send(EngineCommand::Stop) {
                    log!("Error: could not send go command to engine");
                    break;
                }
            }
            (
                Some(UciProtocolOutput::OutputCommandBestMove),
                ParsedInput::EngineMessage(EngineResponse::BestBranch(best_move_result)),
            ) => {
                let best_move_str = match best_move_result {
                    Some(GameMove::Normal(best_move)) => {
                        format!("{}{}", best_move.source, best_move.target)
                    }
                    Some(GameMove::Mate(_)) | None => "0000".to_owned(),
                };
                write_command!("bestmove {}", best_move_str);
            }

            // No action
            (None, _) => continue,
            // Should never come to this
            _ => {
                log!("Unexpected error");
                break;
            }
        }
    }

    let _ = stdin_thread.to_thread.send(());

    let _ = engine_thread.to_thread.send(EngineCommand::Stop);
    let _ = engine_thread.to_thread.send(EngineCommand::Exit);

    let _ = stdin_thread.thread_handle.join();
    let _ = engine_thread.thread_handle.join();
}

fn parse_command(line: &str, logger: &mut Logger) -> Vec<String> {
    let trimmed_buffer = line.trim();
    let elapsed = logger.update();
    let _result = writeln!(logger.file, "> +{} {}", elapsed, trimmed_buffer);
    trimmed_buffer
        .split_whitespace()
        .map(String::from)
        .collect()
}

fn main() {
    std::thread::scope(|scope| uci_loop(scope));
}
