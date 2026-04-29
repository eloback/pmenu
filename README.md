# pmenu

`pmenu` is a Rust password picker with runtime-selectable backends.

It compiles a single binary and selects concrete implementations for menu, password store, clipboard, and autofill at runtime through CLI flags or a config file.

## Features

- Menu backends: `wofi`, `fuzzel`, `bemenu`, `out-gridview`
- Password store backends: `passage`, `pass`, `keepassxc`
- Clipboard backends: `wl-clipboard`, `xclip`, `powershell-clipboard`
- Autofill backends: `wtype`, `powershell-paste`
- Config file support with CLI override precedence
- Reusable `core` library with backend traits and flow orchestration
- Built-in `password`, `username`, `url`, and `fill` selections
- Best-effort context-aware initial search for qutebrowser, Discord, and Steam on Linux

## Project Layout

- `src/core/`: backend traits, domain types, errors, and the main picker flow
- `src/cli/`: CLI parsing, config loading, notifications, and concrete process-backed backends
- `flake.nix`: Nix dev shell and package definition

## Building

With Nix:

```bash
nix develop -c cargo test
nix build .#default
```

Without Nix:

```bash
cargo build
cargo test
```

The compiled binary is named `pmenu`.

## Usage

Run with defaults:

```bash
pmenu
```

Select backends explicitly:

```bash
pmenu \
  --store-backend passage \
  --menu-backend wofi \
  --clipboard-backend wl-clipboard \
  --autofill-backend wtype
```

Trigger autofill instead of copy:

```bash
pmenu --action autofill
```

Select a field directly without opening the second menu:

```bash
pmenu --field fill
```

Enable built-in tracing:

```bash
pmenu --trace
```

## CLI Flags

- `--config <path>`
- `--store-backend <name>`
- `--store-path <path>`
- `--store-identities-file <path>`
- `--store-key-file <path>`
- `--menu-backend <name>`
- `--clipboard-backend <name>`
- `--autofill-backend <name>`
- `--clip-time <seconds>`
- `--field <name>`
- `--action <copy|autofill>`
- `--no-notify`
- `--trace`

CLI flags override config file values.

## Configuration

Default config path:

```text
Linux: ~/.config/pmenu/config.toml
Windows: %APPDATA%\pmenu\config.toml
```

Example:

```toml
[store]
backend = "passage"
path = "~/.passage/store"
identities_file = "~/.passage/identities"

[menu]
backend = "wofi"

[clipboard]
backend = "wl-clipboard"
clip_time_secs = 45

[autofill]
backend = "wtype"
```

Windows KeePassXC example:

```toml
[store]
backend = "keepassxc"
database_path = "C:/Users/example/Passwords.kdbx"
key_file = "C:/Users/example/Passwords.keyx"

[menu]
backend = "out-gridview"

[clipboard]
backend = "powershell-clipboard"

[autofill]
backend = "powershell-paste"
```

Defaults when no config is present on Linux:

- store backend: `passage`
- menu backend: `wofi`
- clipboard backend: `wl-clipboard`
- autofill backend: `wtype`
- action: `copy`
- clip timeout: `45`

Defaults when no config is present on Windows:

- store backend: `keepassxc`
- menu backend: `out-gridview`
- clipboard backend: `powershell-clipboard`
- autofill backend: `powershell-paste`
- action: `copy`
- clip timeout: `45`

## Runtime Notes

- `pass` stores are read from `.gpg` files.
- `passage` stores are read from `.age` files.
- `keepassxc` stores are read from a configured `.kdbx` database through `keepassxc-cli`.
- File-based stores skip hidden files and directories.
- The picker exposes built-in `username` and `url` fields derived from entry metadata or the entry path.
- The `fill` option types username, then tab, then password when an autofill backend is available.
- On Linux, `wofi` can open with an initial search based on the focused app; qutebrowser, Discord, and Steam are recognized.
- `~` is expanded in configured paths.
- `store.path` and `store.database_path` are interchangeable for `keepassxc`; `store.database_path` is clearer.
- Notifications use `notify-send` on Linux when enabled.
- Windows copy/autofill backends require PowerShell with desktop UI support.
- `out-gridview` requires a Windows desktop session.
- `--trace` emits detailed logs to stderr without printing secret values.
- `RUST_LOG` can be used instead of `--trace` for custom log filtering.
- Missing external tools fail at runtime with a clear command-specific error.
