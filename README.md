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

Squadron checks for updates on start. When one is out, the status bar shows **Update**: one click downloads it, installs it and restarts.

## Develop

```bash
pnpm install
pnpm tauri dev
```

`pnpm check` runs the type checks, the tests, rustfmt and clippy.

## Release

`scripts/release.sh <version> [notes]` builds a signed AppImage, publishes the source and the release on GitHub, and the apps already installed pick it up. It needs the signing key in `~/.tauri/squadron.key`.
