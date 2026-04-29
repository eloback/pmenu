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
    )?;

    let clipboard = match config.action {
        AppAction::Copy => Some(backends::clipboard::build(
            &config.clipboard_backend,
            config.clip_time_secs,
        )?),
        AppAction::Autofill => None,
    };

    let autofill = build_autofill_backend(config.action, &config.autofill_backend)?;

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

fn build_autofill_backend(
    action: AppAction,
    backend_name: &str,
) -> Result<Option<Box<dyn crate::core::AutofillBackend>>, AppError> {
    let backend = backends::autofill::build(backend_name)?;

    if matches!(action, AppAction::Autofill) {
        Ok(Some(backend))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::build_autofill_backend;
    use crate::core::{AppAction, AppError};

    #[test]
    fn copy_mode_validates_backend_but_disables_fill() {
        let backend = build_autofill_backend(AppAction::Copy, "wtype")
            .expect("copy mode should still validate valid backend names");

        assert!(backend.is_none());
    }

    #[test]
    fn copy_mode_reports_invalid_autofill_backend() {
        let error = match build_autofill_backend(AppAction::Copy, "wtpye") {
            Ok(_) => panic!("invalid backend should surface even in copy mode"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            AppError::Config(message) if message == "Unknown autofill backend: wtpye"
        ));
    }

    #[test]
    fn autofill_mode_reports_invalid_autofill_backend() {
        let error = match build_autofill_backend(AppAction::Autofill, "wtpye") {
            Ok(_) => panic!("invalid backend should surface in autofill mode"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            AppError::Config(message) if message == "Unknown autofill backend: wtpye"
        ));
    }
}
