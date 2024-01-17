use std::time::Duration;

use crate::duplex_thread::{create_duplex_thread, DuplexThread};

pub type StdinResponse = Result<String, String>;

fn stdin_thread(
    stop_signal: crossbeam_channel::Receiver<()>,
    stdin_lines_sender: crossbeam_channel::Sender<StdinResponse>,
) {
    let stdin = async_std::io::stdin();
    loop {
        if stop_signal.try_recv().is_ok() {
            break;
        }

        let try_to_read_stdin = async_std::io::timeout(Duration::from_millis(100), async {
            let mut line = String::new();
            let _ = stdin.read_line(&mut line).await;
            Ok(line)
        });
        match async_std::task::block_on(try_to_read_stdin) {
            Ok(line) => {
                let _ = stdin_lines_sender.send(Ok(line));
            }
            Err(err) => {
                if err.kind() == async_std::io::ErrorKind::TimedOut {
                    continue;
                }
                let _ = stdin_lines_sender.send(Err(format!("{err}")));
                break;
            }
        }
    }
}

pub fn create_stdin_thread<'scope>(
    scope: &'scope std::thread::Scope<'scope, '_>,
) -> DuplexThread<'scope, (), StdinResponse> {
    create_duplex_thread(scope, stdin_thread)
}
