use tokio::task::JoinHandle;

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}", e)?;
    let mut source = e.source();
    while let Some(cause) = source {
        writeln!(f, "Caused by: {}", cause)?;
        source = cause.source();
    }
    Ok(())
}

/// NOTE: awaiting this will return a result, so if your inner closure returns a result it will be
/// nested
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    tokio::task::spawn_blocking(move || span.in_scope(f))
}
