# Squadron

A desktop app to drive several Claude Code agents side by side: one tab per project, agents with their own conversations, permission cards, diffs, a terminal and a Kanban.

## Install (Linux)

Install [Claude Code](https://docs.claude.com/en/docs/claude-code/setup) and log in once in a terminal with `claude auth login`. Then:

```bash
curl -fsSL https://raw.githubusercontent.com/Jgrasp/squadron/main/install.sh | bash
```

It puts the latest AppImage in `~/.local/bin` and adds Squadron to your applications. Squadron checks for updates every ten minutes: when one is out, the status bar shows **Update**, and one click downloads it, installs it and restarts.

If Squadron does not start, your system may miss FUSE, which AppImages need: `sudo apt install libfuse2t64` on Ubuntu 24.04 and later, `libfuse2` before.

## Develop

```bash
pnpm install
pnpm tauri dev
```

`pnpm check` runs the type checks, the tests, rustfmt and clippy.

## Release

`scripts/release.sh <version> [notes]` sets the version, publishes the source and pushes the `v<version>` tag. GitHub Actions then builds the signed AppImage and the release, and the apps already installed pick it up within ten minutes. The signing key lives in the repository secret `TAURI_SIGNING_PRIVATE_KEY`.
