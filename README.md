# Squadron

A desktop app to drive several Claude Code agents side by side: a tab for each pinned or recently active project, a panel that lists them all in groups, agents with their own conversations, permission cards, diffs, a terminal and a Kanban.

## Install

Install [Claude Code](https://docs.claude.com/en/docs/claude-code/setup) and log in once in a terminal with `claude auth login`. Then, on Linux or macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/akki-team/squadron/main/install.sh | bash
```

**Linux.** On Ubuntu and Debian it installs the `.deb` package, which uses the system's WebKit, and asks for your password. Elsewhere it puts the AppImage in `~/.local/bin` and adds Squadron to your applications; AppImages need FUSE (`libfuse2`).

**macOS.** It puts Squadron in `/Applications`, for Apple Silicon and Intel Macs. You can also download the `.dmg` from the [latest release](https://github.com/akki-team/squadron/releases/latest): Squadron is not notarized by Apple, so on first launch macOS refuses to open it; open **System Settings › Privacy & Security** and click **Open Anyway**, once.

Squadron checks for updates every ten minutes, and on demand from its menu with **Check for updates**: when one is out, the status bar shows **Update**, and one click downloads it, installs it and restarts. The `.deb` asks for your password to install the update.

## Pull requests

To follow pull requests, Squadron needs the extension of each project's forge. **Settings › Pull requests** offers the GitHub, Bitbucket, GitLab or Azure DevOps extension for a project on github.com, bitbucket.org, gitlab.com or dev.azure.com; install it, then add an account to it: a personal access token for GitHub, your Atlassian email and an API token for Bitbucket, the address of your GitLab and a personal access token for GitLab, your organization and a personal access token for Azure DevOps.

## Develop

```bash
pnpm install
pnpm app
```

The dev build keeps its own data (`dev.squadron.desktop.dev`) and keyring entry, apart from the installed Squadron, so both can run side by side. Its first start copies the projects, settings and layout of the installed one; agents and tokens are not copied.

`pnpm check` runs the type checks, the tests, rustfmt and clippy.

## Release

Every change reaches `main` through a pull request, which can only be merged once its `test` and `secrets` checks pass. `scripts/release.sh <version> [notes] [fr=notes]` opens the pull request "Squadron <version>" with the new version and changelog. Once it is merged, `scripts/release.sh tag <version>` pushes the `v<version>` tag: GitHub Actions tests again, then builds the Linux packages (`.deb`, AppImage), the macOS ones (`.dmg`, universal app) and the Windows installer with their update signatures, and the apps already installed pick the release up within ten minutes. The signing key lives in the repository secret `TAURI_SIGNING_PRIVATE_KEY`. A release with a new feature bumps the minor version; one made of fixes only bumps the patch.
