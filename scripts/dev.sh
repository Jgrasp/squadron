#!/usr/bin/env bash
exec >"$HOME/.cache/squadron-dev.log" 2>&1
export NVM_DIR="$HOME/.nvm"
. "$NVM_DIR/nvm.sh"
. "$HOME/.cargo/env"
cd "$(dirname "$0")/.."
exec pnpm app
