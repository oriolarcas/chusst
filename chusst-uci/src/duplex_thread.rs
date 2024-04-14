use std::{
    future::Future,
    io::Error as IoError,
    thread::{self, JoinHandle},
};

use tokio::{
    runtime::Builder,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

pub struct DuplexChannel<ToThreadType, FromThreadType> {
    pub to_thread: UnboundedSender<ToThreadType>,
    pub from_thread: UnboundedReceiver<FromThreadType>,
}

pub struct DuplexThread<Output, ToThreadType, FromThreadType> {
    pub thread_handle: JoinHandle<Result<Output, IoError>>,
    pub channel: DuplexChannel<ToThreadType, FromThreadType>,
}

pub fn create_duplex_thread<F, Fut, Input, ToThreadType, FromThreadType>(
    name: &str,
    entry: F,
    init: Input,
) -> DuplexThread<Fut::Output, ToThreadType, FromThreadType>
where
    F: FnOnce(
            Input,
            mpsc::UnboundedReceiver<ToThreadType>,
            mpsc::UnboundedSender<FromThreadType>,
        ) -> Fut
        + Send
        + 'static,
    Fut: Future + Send + 'static,
    Fut::Output: Send,
    Input: Send + 'static,
    ToThreadType: Send + 'static,
    FromThreadType: Send + 'static,
{
    let (to_thread_send, to_thread_receive) = mpsc::unbounded_channel::<ToThreadType>();
    let (from_thread_send, from_thread_receive) = mpsc::unbounded_channel::<FromThreadType>();

    let name = name.to_string();

    // Spawn thread
    let thread = thread::spawn(move || {
        let result = Builder::new_current_thread()
            .enable_all()
            .thread_name(name.clone())
            .build()?
            .block_on(async move {
                let task = entry(init, to_thread_receive, from_thread_send);
                #[cfg(feature = "tokio-console")]
                let result = tokio::task::Builder::new()
                    .name(&name)
                    .spawn(task)
                    .unwrap()
                    .await
                    .unwrap();
                #[cfg(not(feature = "tokio-console"))]
                let result = task.await;
                result
            });
        Ok::<Fut::Output, IoError>(result)
    });

    DuplexThread {
        thread_handle: thread,
        channel: DuplexChannel {
            to_thread: to_thread_send,
            from_thread: from_thread_receive,
        },
    }
}
