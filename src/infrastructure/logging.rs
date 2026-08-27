use std::sync::OnceLock;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static VERBOSE: OnceLock<bool> = OnceLock::new();

pub fn init(verbose: bool) {
    let _ = VERBOSE.set(verbose);

    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}

/// 是否运行在 verbose 诊断模式。
pub fn verbose() -> bool {
    *VERBOSE.get().unwrap_or(&false)
}
