#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="${HOME}/.dotnet:${PATH}"
(cd "$root/server" && cargo run) &
server_pid=$!
trap 'kill $server_pid 2>/dev/null || true' EXIT
cd "$root/client"
dotnet run
