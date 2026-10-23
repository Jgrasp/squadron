# Squadron

A desktop app to drive several Claude Code agents side by side: one tab per project, agents with their own conversations, permission cards, diffs, a terminal and a Kanban.

## Install (Linux)

Install [Claude Code](https://docs.claude.com/en/docs/claude-code/setup) and log in once in a terminal with `claude auth login`. Then:

```bash
curl -fsSL https://raw.githubusercontent.com/Jgrasp/squadron/main/install.sh | bash
```

On Ubuntu and Debian it installs the `.deb` package, which uses the system's WebKit, and asks for your password. Elsewhere it puts the AppImage in `~/.local/bin` and adds Squadron to your applications; AppImages need FUSE (`libfuse2`).

Squadron checks for updates every ten minutes: when one is out, the status bar shows **Update**, and one click downloads it, installs it and restarts. The `.deb` asks for your password to install the update.

## Develop

```bash
pnpm install
pnpm tauri dev
```

`pnpm check` runs the type checks, the tests, rustfmt and clippy.

## Release

`scripts/release.sh <version> [notes]` sets the version, publishes the source and pushes the `v<version>` tag. GitHub Actions then builds the signed AppImage and the release, and the apps already installed pick it up within ten minutes. The signing key lives in the repository secret `TAURI_SIGNING_PRIVATE_KEY`.
