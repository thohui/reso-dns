#!/usr/bin/env bash

# Development script that runs both the server and UI together.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
web="$root/reso/web"

if [[ ! -d "$web/node_modules" ]]; then
  pnpm --dir "$web" install
fi

cleanup() {
  trap - EXIT INT TERM
  kill 0 2>/dev/null || true
}
trap cleanup EXIT INT TERM

(cd "$root/reso" && CARGO_TERM_COLOR=always exec cargo run -p reso) \
  > >(sed -u 's/^/[api] /') 2>&1 &

FORCE_COLOR=1 pnpm --dir "$web" dev \
  > >(sed -u 's/^/[web] /') 2>&1 &

wait -n
