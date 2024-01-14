mod duplex_thread;
mod engine;
mod stdin;

use chusst_gen::eval::GameMove;
use chusst_gen::game::{BitboardGame, MoveAction};
use engine::{create_engine_thread, EngineCommand, EngineResponse, GoCommand, NewGameCommand};
use stdin::{create_stdin_thread, StdinResponse};

use crossbeam_channel::select;
use rust_fsm::*;

use std::fmt;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

enum SyncInput {
    FromStdin(StdinResponse),
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

    Initializing(CommandUci) => Ready [OutputCommandId],
    Ready => {
        CommandGo => Searching [EngineCommandGo],
        CommandPosition => Ready [EngineCommandPosition],
        // Additional commands
        CommandUciNewGame => Ready [EngineCommandNewGame],
        CommandIsReady => Ready [OutputCommandReadyOk],
        CommandSetOption => Ready [EngineCommandSetOption],
        CommandStop => Ready [OutputSavedCommandBestMove],
    },
    Searching => {
        CommandStop => WaitingForResult [EngineCommandStop],
        EngineInfo => Searching [OutputCommandInfo],
        EngineResult => Ready [SaveBestMove],
        // Additional commands
        CommandIsReady => Ready [OutputCommandReadyOk],
    },
    WaitingForResult => {
        EngineInfo => WaitingForResult [OutputCommandInfo],
        EngineResult => Ready [OutputCommandBestMove],
        // Additional commands
        CommandIsReady => Ready [OutputCommandReadyOk],
        CommandStop => WaitingForResult,
    },
}

enum ParsedInput {
    UciStdInInput(Vec<String>),
    EngineMessage(EngineResponse),
}

const SEARCH_DEPTH_DEFAULT: u32 = 4;
const SEARCH_DEPTH_MIN: u32 = 2;
const SEARCH_DEPTH_MAX: u32 = 5;

impl fmt::Display for UciProtocolState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = match self {
            UciProtocolState::Initializing => "Initializing",
            UciProtocolState::Ready => "Ready",
            UciProtocolState::Searching => "Searching",
            UciProtocolState::WaitingForResult => "WaitingForResult",
        };
        write!(f, "{}", state)
    }
}

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
    let engine_thread = create_engine_thread(scope);
    let stdin_thread = create_stdin_thread(scope);

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

    let get_input = || -> Result<SyncInput, String> {
        select! {
            recv(stdin_thread.from_thread) -> stdin_line => return Ok(SyncInput::FromStdin(stdin_line.map_err(stringify_stdin_err)?)),
            recv(engine_thread.from_thread) -> engine_response => return Ok(SyncInput::FromEngine(engine_response.map_err(stringify_engine_err)?)),
        }
    };

    let mut last_best_move: Option<Option<GameMove>> = None;

    loop {
        let input = get_input();

        let (protocol_input, parsed_input) = match &input {
            Ok(SyncInput::FromStdin(stdin_response)) => match &stdin_response {
                Ok(stdin_line) => {
                    let words = parse_command(stdin_line.as_str(), &mut logger);
                    let protocol_stdin_input = if let Some(stdin_command) = words.first() {
                        match stdin_command.as_str() {
                            "uci" => UciProtocolInput::CommandUci,
                            "isready" => UciProtocolInput::CommandIsReady,
                            "setoption" => UciProtocolInput::CommandSetOption,
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
                Err(error_message) => {
                    log!("stdin error: {error_message}");
                    log!("Exiting");
                    break;
                }
            },
            Ok(SyncInput::FromEngine(engine_response)) => {
                let engine_protocol_input = match &engine_response {
                    EngineResponse::Log(message) => {
                        let trimmed_message = message.trim();
                        if !trimmed_message.is_empty() {
                            log!("(engine) {}", message.trim());
                        }
                        continue;
                    }
                    EngineResponse::Ready => continue,
                    EngineResponse::Info(_) => UciProtocolInput::EngineInfo,
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
            Err(error_message) => {
                log!("I/O error: {error_message}");
                log!("Exiting");
                break;
            }
        };

        let previous_state = uci_protocol.state().to_string();

        let protocol_output_result = uci_protocol.consume(&protocol_input);

        let protocol_output = match &protocol_output_result {
            Ok(output) => output,
            Err(_) => {
                log!("Unexpected input, ignoring");
                continue;
            }
        };

        let current_state = uci_protocol.state().to_string();

        if current_state != previous_state {
            log!(
                "New state: {} -> {}",
                previous_state,
                uci_protocol.state().to_string()
            );
        }

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
            (
                Some(UciProtocolOutput::EngineCommandSetOption),
                ParsedInput::UciStdInInput(words),
            ) => {
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
                            game: Some(BitboardGame::new()),
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
                    Some("startpos") => (param_iter.next(), Some(BitboardGame::new())),
                    Some("fen") => {
                        log!("Parsing FEN string...");

                        let fen: Vec<&str> = [
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                            param_iter.next(),
                        ]
                        .iter()
                        .map_while(|token| *token)
                        .collect();

                        if let Some(new_game_from_fen) = BitboardGame::try_from_fen(fen.as_slice()) {
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
                if let Some(game) = &new_game {
                    let _ = log!("New position:\n{}", game.board());
                }
                let mut new_game_command = NewGameCommand {
                    game: new_game,
                    moves: Vec::new(),
                };
                match next_token {
                    Some("moves") => {
                        for mv_str in param_iter {
                            if let Some(mv) = MoveAction::try_from_long_algebraic_str(&mv_str) {
                                new_game_command.moves.push(mv);
                            } else {
                                log!("Malformed move in position command");
                                continue;
                            }
                        }
                    }
                    Some(_) => {
                        log!("Malformed move in position command");
                        continue;
                    }
                    None => (),
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
                // go infinite
                //   Search until stop is received
                // go wtime 300000 btime 300000 movestogo 40
                //   Search with this amount of time
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
                    _ => {
                        log!("Error: Unknown go command");
                        break;
                    }
                }
            }
            (
                Some(UciProtocolOutput::SaveBestMove),
                ParsedInput::EngineMessage(EngineResponse::BestBranch(best_move_result)),
            ) => {
                let best_move_str = move_to_uci_string(&best_move_result);
                write_command!("bestmove {}", best_move_str);
                last_best_move = Some(best_move_result);
            }
            (Some(UciProtocolOutput::EngineCommandStop), ParsedInput::UciStdInInput(_)) => {
                if let Err(_) = engine_thread.to_thread.send(EngineCommand::Stop) {
                    log!("Error: could not send go command to engine");
                    break;
                }
            }
            (
                Some(UciProtocolOutput::OutputSavedCommandBestMove),
                ParsedInput::UciStdInInput(_),
            ) => {
                if let Some(best_move) = &last_best_move {
                    let best_move_str = move_to_uci_string(&best_move);
                    write_command!("bestmove {}", best_move_str);
                }
            }
            (
                Some(UciProtocolOutput::OutputCommandInfo),
                ParsedInput::EngineMessage(EngineResponse::Info(info)),
            ) => {
                write_command!(
                    "info depth {} nodes {} score cp {}",
                    info.depth,
                    info.nodes,
                    info.score
                );
            }
            (
                Some(UciProtocolOutput::OutputCommandBestMove),
                ParsedInput::EngineMessage(EngineResponse::BestBranch(best_move_result)),
            ) => {
                let best_move_str = move_to_uci_string(&best_move_result);
                write_command!("bestmove {}", best_move_str);
            }

            // No action
            (None, _) => continue,
            // Should never come to this
            _ => {
                log!("Unexpected protocol state");
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

fn stringify_stdin_err<T>(value: T) -> String
where
    T: std::fmt::Display,
{
    stringify(value, "stdin")
}

fn stringify_engine_err<T>(value: T) -> String
where
    T: std::fmt::Display,
{
    stringify(value, "engine")
}

fn stringify<T>(value: T, prefix: &str) -> String
where
    T: std::fmt::Display,
{
    format!("{prefix}: {value}")
}

fn move_to_uci_string(mv: &Option<GameMove>) -> String {
    match mv {
        Some(GameMove::Normal(best_move)) => {
            format!("{}{}", best_move.mv.source, best_move.mv.target)
        }
        Some(GameMove::Mate(_)) | None => "0000".to_owned(),
    }
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
