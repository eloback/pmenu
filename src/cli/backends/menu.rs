use std::process::{Command, Stdio};

use crate::core::{AppError, MenuBackend};

pub fn build(name: &str) -> Result<Box<dyn MenuBackend>, AppError> {
    match name {
        "fuzzel" => Ok(Box::new(FuzzelMenu)),
        "bemenu" => Ok(Box::new(BemenuMenu)),
        "wofi" => Ok(Box::new(WofiMenu)),
        _ => Err(AppError::Config(format!("Unknown menu backend: {name}"))),
    }
}

struct FuzzelMenu;
struct BemenuMenu;
struct WofiMenu;

impl MenuBackend for FuzzelMenu {
    fn select(
        &self,
        prompt: &str,
        items: &[String],
        initial_query: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        run_menu_command("fuzzel", &fuzzel_args(prompt, initial_query), items)
    }
}

impl MenuBackend for BemenuMenu {
    fn select(
        &self,
        prompt: &str,
        items: &[String],
        initial_query: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        run_menu_command("bemenu", &bemenu_args(prompt, initial_query), items)
    }
}

impl MenuBackend for WofiMenu {
    fn select(
        &self,
        prompt: &str,
        items: &[String],
        initial_query: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        run_menu_command("wofi", &wofi_args(prompt, initial_query), items)
    }
}

fn run_menu_command(
    program: &str,
    args: &[String],
    items: &[String],
) -> Result<Option<String>, AppError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| command_error(program, error))?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        for item in items {
            stdin.write_all(item.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
    }
    drop(child.stdin.take());

    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if output.status.success() {
        return if stdout.is_empty() {
            Ok(None)
        } else {
            Ok(Some(stdout))
        };
    }

    if matches!(output.status.code(), Some(1) | Some(130)) && stdout.is_empty() {
        return Ok(None);
    }

    Err(AppError::CommandFailed {
        command: format!("{program} {}", args.join(" ")).trim().to_string(),
        code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn fuzzel_args(prompt: &str, _initial_query: Option<&str>) -> Vec<String> {
    vec![
        "--dmenu".to_string(),
        "--lines".to_string(),
        "10".to_string(),
        "--font".to_string(),
        "FiraCode Nerd Font:size=9".to_string(),
        "--background".to_string(),
        "32302fff".to_string(),
        "--text-color".to_string(),
        "ddc7a1ff".to_string(),
        "--prompt-color".to_string(),
        "a9b665ff".to_string(),
        "--input-color".to_string(),
        "ebdbb2ff".to_string(),
        "--match-color".to_string(),
        "D8A657ff".to_string(),
        "--selection-color".to_string(),
        "504945ff".to_string(),
        "--selection-text-color".to_string(),
        "ebdbb2ff".to_string(),
        "--selection-match-color".to_string(),
        "D8A657ff".to_string(),
        "--border-color".to_string(),
        "32302fff".to_string(),
        "--prompt".to_string(),
        prompt.to_string(),
    ]
}

fn bemenu_args(prompt: &str, _initial_query: Option<&str>) -> Vec<String> {
    vec![
        "--nb".to_string(),
        "#32302F".to_string(),
        "--nf".to_string(),
        "#ddc7a1".to_string(),
        "--sb".to_string(),
        "#32302f".to_string(),
        "--sf".to_string(),
        "#ebdbb2".to_string(),
        "--hb".to_string(),
        "#D8A657".to_string(),
        "--hf".to_string(),
        "#292828".to_string(),
        "--fb".to_string(),
        "#504945".to_string(),
        "--ff".to_string(),
        "#bdae93".to_string(),
        "--tb".to_string(),
        "#504945".to_string(),
        "--tf".to_string(),
        "#a9b665".to_string(),
        "--cb".to_string(),
        "#a9b665".to_string(),
        "--cf".to_string(),
        "#EA6962".to_string(),
        "--bdr".to_string(),
        "#32302F".to_string(),
        "--ignorecase".to_string(),
        "-p".to_string(),
        prompt.to_string(),
        "-l".to_string(),
        "10".to_string(),
        "--border=0".to_string(),
        "--fn".to_string(),
        "FiraCode Nerd Font 9".to_string(),
    ]
}

fn wofi_args(prompt: &str, initial_query: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "--dmenu".to_string(),
        "--prompt".to_string(),
        prompt.to_string(),
        "--lines".to_string(),
        "10".to_string(),
        "--insensitive".to_string(),
    ];

    if let Some(initial_query) = initial_query.filter(|value| !value.is_empty()) {
        args.push("--search".to_string());
        args.push(initial_query.to_string());
    }

    args
}

fn command_error(program: &str, error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AppError::CommandMissing(program.to_string()),
        _ => AppError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{bemenu_args, fuzzel_args, wofi_args};

    #[test]
    fn fuzzel_args_include_prompt() {
        let args = fuzzel_args("prompt", None);
        assert!(args.ends_with(&["--prompt".to_string(), "prompt".to_string()]));
    }

    #[test]
    fn bemenu_args_include_prompt() {
        let args = bemenu_args("prompt", None);
        assert!(args.contains(&"prompt".to_string()));
    }

    #[test]
    fn wofi_args_include_dmenu_mode() {
        let args = wofi_args("prompt", Some("example.com"));
        assert!(args.contains(&"--dmenu".to_string()));
        assert!(args.contains(&"prompt".to_string()));
        assert!(args.contains(&"--search".to_string()));
        assert!(args.contains(&"example.com".to_string()));
    }
}
