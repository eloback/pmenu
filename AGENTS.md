# Agent Notes

This repository is a small Rust CLI application named `pmenu`.

## Architecture

- Keep `src/core/` free of direct shell-command and platform-specific logic.
- Put reusable traits and orchestration in `core`.
- Put config parsing, backend selection, notifications, and command execution in `src/cli/`.
- Prefer adding new behavior by extending backend traits or adding CLI adapters, not by pushing process logic into `core`.

## Current Runtime Model

- One compiled binary contains all supported backends.
- Backend choice is runtime-selected by config and/or CLI flags.
- CLI flags override config file values.
- Config file path defaults to `~/.config/pmenu/config.toml`.

Supported backend names:

- Menus: `wofi`, `fuzzel`, `bemenu`
- Stores: `passage`, `pass`
- Clipboard: `wl-clipboard`, `xclip`
- Autofill: `wtype`

## Development Conventions

- Preserve the `pmenu` package and binary name.
- Keep backend names lowercase and stable; treat them as public config/CLI identifiers.
- When adding config fields, wire them through:
  - CLI args
  - config structs
  - resolved runtime config
  - backend construction
- Prefer explicit, user-facing `AppError::Config` messages for invalid config and unknown backends.

## Validation

Preferred checks:

```bash
nix develop -c cargo test
nix build .#default
```

If Nix is unavailable:

```bash
cargo test
```

## Nix

- `flake.nix` should keep the dev shell and package aligned with the runtime backend set.
- If a new external backend is added, add its system package to both:
  - `runtimeTools` in `packages`
  - `runtimeTools` in `devShells`

## Docs

- Update `README.md` when changing:
  - backend names
  - CLI flags
  - config file shape
  - default backends
