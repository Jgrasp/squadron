# Squadron

A desktop app to drive several Claude Code agents side by side: one tab per project, agents with their own conversations, permission cards, diffs, a terminal and a Kanban.

## Install (Linux)

1. Install [Claude Code](https://docs.claude.com/en/docs/claude-code/setup) and log in once in a terminal: `claude auth login`.
2. Download the latest `squadron_*_amd64.AppImage` from the [releases](https://github.com/Jgrasp/squadron/releases/latest).
3. Make it executable and start it:

   ```bash
   chmod +x squadron_*_amd64.AppImage
   ./squadron_*_amd64.AppImage
   ```

Squadron checks for updates every ten minutes. When one is out, the status bar shows **Update**: one click downloads it, installs it and restarts.

## Develop

```bash
pnpm install
pnpm tauri dev
```

`pnpm check` runs the type checks, the tests, rustfmt and clippy.

## Release

`scripts/release.sh <version> [notes]` sets the version, publishes the source and pushes the `v<version>` tag. GitHub Actions then builds the signed AppImage and the release, and the apps already installed pick it up within ten minutes. The signing key lives in the repository secret `TAURI_SIGNING_PRIVATE_KEY`.
