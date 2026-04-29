use tracing_subscriber::EnvFilter;

pub fn init(trace_enabled: bool) {
    if !trace_enabled && std::env::var_os("RUST_LOG").is_none() {
        return;
    }

    let env_filter = if std::env::var_os("RUST_LOG").is_some() {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("pmenu=trace"))
    } else if trace_enabled {
        EnvFilter::new("pmenu=trace")
    } else {
        return;
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(false)
        .compact()
        .try_init();
}
