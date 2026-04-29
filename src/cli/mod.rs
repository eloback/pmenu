mod args;
mod backends;
mod config;
mod logging;
mod notify;

use clap::Parser;
use tracing::{debug, trace};

use crate::core::{run_flow, AppAction, AppError};

pub fn run() -> Result<(), AppError> {
    let args = args::CliArgs::parse();
    logging::init(args.trace);

    let config = config::ResolvedConfig::load(args)?;
    debug!(?config, "resolved runtime config");

    let menu = backends::menu::build(&config.menu_backend)?;
    let store = backends::store::build(
        &config.store_backend,
        config.store_path.clone(),
        config.store_identities_file.clone(),
        config.store_key_file.clone(),
    )?;

    let clipboard = match config.action {
        AppAction::Copy => Some(backends::clipboard::build(
            &config.clipboard_backend,
            config.clip_time_secs,
        )?),
        AppAction::Autofill => None,
    };

    let autofill = match config.action {
        AppAction::Copy => None,
        AppAction::Autofill => Some(backends::autofill::build(&config.autofill_backend)?),
    };

    let outcome = run_flow(
        menu.as_ref(),
        store.as_ref(),
        clipboard.as_deref(),
        autofill.as_deref(),
        config.action,
    )?;
    trace!(completed = outcome.is_some(), "completed application flow");

    if let Some(outcome) = outcome {
        notify::Notifier::new(config.notify)
            .notify(outcome.action.past_tense(), &outcome.field_name);
    }

    Ok(())
}
