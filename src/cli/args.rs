use clap::{Parser, ValueEnum};

use crate::core::AppAction;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "pmenu",
    version,
    about = "Password picker with runtime-selectable backends"
)]
pub struct CliArgs {
    #[arg(long, value_name = "PATH")]
    pub config: Option<String>,

    #[arg(long, value_name = "NAME")]
    pub store_backend: Option<String>,

    #[arg(long, value_name = "PATH")]
    pub store_path: Option<String>,

    #[arg(long, value_name = "PATH")]
    pub store_identities_file: Option<String>,

    #[arg(long, value_name = "PATH")]
    pub store_key_file: Option<String>,

    #[arg(long, value_name = "NAME")]
    pub menu_backend: Option<String>,

    #[arg(long, value_name = "NAME")]
    pub clipboard_backend: Option<String>,

    #[arg(long, value_name = "NAME")]
    pub autofill_backend: Option<String>,

    #[arg(long, value_name = "SECONDS")]
    pub clip_time: Option<u64>,

    #[arg(long, value_name = "NAME")]
    pub field: Option<String>,

    #[arg(long, value_enum)]
    pub action: Option<CliAction>,

    #[arg(long)]
    pub no_notify: bool,

    #[arg(long)]
    pub trace: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliAction {
    Copy,
    Autofill,
}

impl From<CliAction> for AppAction {
    fn from(value: CliAction) -> Self {
        match value {
            CliAction::Copy => AppAction::Copy,
            CliAction::Autofill => AppAction::Autofill,
        }
    }
}
