# pmenu

`pmenu` is a Rust password picker with runtime-selectable backends.

It compiles a single binary and selects concrete implementations for menu, password store, clipboard, and autofill at runtime through CLI flags or a config file.

## Features

- Menu backends: `wofi`, `fuzzel`, `bemenu`
- Password store backends: `passage`, `pass`
- Clipboard backends: `wl-clipboard`, `xclip`
- Autofill backends: `wtype`
- Config file support with CLI override precedence
- Reusable `core` library with backend traits and flow orchestration

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

## CLI Flags

- `--config <path>`
- `--store-backend <name>`
- `--store-path <path>`
- `--store-identities-file <path>`
- `--menu-backend <name>`
- `--clipboard-backend <name>`
- `--autofill-backend <name>`
- `--clip-time <seconds>`
- `--action <copy|autofill>`
- `--no-notify`

CLI flags override config file values.

## Configuration

Default config path:

```text
~/.config/pmenu/config.toml
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

Defaults when no config is present:

- store backend: `passage`
- menu backend: `wofi`
- clipboard backend: `wl-clipboard`
- autofill backend: `wtype`
- action: `copy`
- clip timeout: `45`

## Runtime Notes

- `pass` stores are read from `.gpg` files.
- `passage` stores are read from `.age` files.
- `~` is expanded in configured paths.
- Notifications use `notify-send` when enabled.
- Missing external tools fail at runtime with a clear command-specific error.
