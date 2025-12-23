// Initialize tracing once for all tests
pub static TRACING: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    let default_filter = "info";
    let filter = std::env::var("TEST_LOG").unwrap_or_else(|_| default_filter.to_string());

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .with_env_filter(filter)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
});
