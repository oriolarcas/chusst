pub struct DuplexThread<'scope, ToThreadType, FromThreadType> {
    pub thread_handle: std::thread::ScopedJoinHandle<'scope, ()>,
    pub to_thread: crossbeam_channel::Sender<ToThreadType>,
    pub from_thread: crossbeam_channel::Receiver<FromThreadType>,
}

pub fn create_duplex_thread<'scope, 'env, F, ToThreadType, FromThreadType>(
    scope: &'scope std::thread::Scope<'scope, 'env>,
    entry: F,
) -> DuplexThread<'scope, ToThreadType, FromThreadType>
where
    F: FnOnce(crossbeam_channel::Receiver<ToThreadType>, crossbeam_channel::Sender<FromThreadType>)
        + Send + 'scope,
    ToThreadType: Send + 'scope,
    FromThreadType: Send + 'scope,
{
    let (to_thread_send, to_thread_receive) = crossbeam_channel::unbounded::<ToThreadType>();
    let (from_thread_send, from_thread_receive) = crossbeam_channel::unbounded::<FromThreadType>();

    // Spawn thread
    let thread = scope.spawn(move || entry(to_thread_receive, from_thread_send));

    DuplexThread {
        thread_handle: thread,
        to_thread: to_thread_send,
        from_thread: from_thread_receive,
    }
}
