mod args;
mod backends;
mod config;
mod context;
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
    let initial_query = context::initial_query();

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

    let autofill = match backends::autofill::build(&config.autofill_backend) {
        Ok(backend) => Some(backend),
        Err(error) if matches!(config.action, AppAction::Copy) => {
            trace!(?error, "autofill backend unavailable; disabling fill option");
            None
        }
        Err(error) => return Err(error),
    };

    let outcome = run_flow(
        menu.as_ref(),
        store.as_ref(),
        clipboard.as_deref(),
        autofill.as_deref(),
        config.action,
        initial_query.as_deref(),
        config.field.as_deref(),
    )?;
    trace!(completed = outcome.is_some(), "completed application flow");

    if let Some(outcome) = outcome {
        notify::Notifier::new(config.notify)
            .notify(outcome.action.past_tense(), &outcome.field_name);
    }

    Ok(())
}
