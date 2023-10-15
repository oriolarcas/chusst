use std::time::Duration;

use crate::duplex_thread::{create_duplex_thread, DuplexThread};

fn stdin_thread(
    stop_signal: crossbeam_channel::Receiver<()>,
    stdin_lines_sender: crossbeam_channel::Sender<String>,
) {
    loop {
        if stop_signal.try_recv().is_ok() {
            break;
        }

        let try_to_read_stdin = async_std::io::timeout(Duration::from_millis(100), async {
            let mut line = String::new();
            let _ = async_std::io::stdin().read_line(&mut line).await;
            Ok(line)
        });
        match async_std::task::block_on(try_to_read_stdin) {
            Ok(line) => {
                let _ = stdin_lines_sender.send(line);
            }
            Err(_) => break,
        }
    }
}

pub fn create_stdin_thread<'scope, 'env>(scope: &'scope std::thread::Scope<'scope, 'env>) -> DuplexThread<'scope, (), String> {
    create_duplex_thread(scope, stdin_thread)
}
