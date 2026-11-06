#!/usr/bin/env bash
# Runs the web frontend as the live site on :3000 (what librarian.dastari.net points at).
#
#   ./live.sh start [--preview]   dev server (default) or production build served by `vite preview`
#   ./live.sh stop | restart [--preview] | status | logs
#
# Backend address and public origin come from .env.local (BACKEND_PROXY_TARGET, DEV_SERVER_PUBLIC_URL).
set -euo pipefail
cd "$(dirname "$0")"
PORT="${PORT:-3000}"
PIDFILE=/tmp/librarian-web-live.pid
LOG=/tmp/librarian-web-live.log
UNIT=librarian-web

use_systemd() { [ "$(id -u)" = 0 ] && [ -d /run/systemd/system ] && command -v systemd-run > /dev/null; }

running() {
  if use_systemd && systemctl is-active --quiet "$UNIT"; then return 0; fi
  [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null
}

pid() {
  if use_systemd && systemctl is-active --quiet "$UNIT"; then systemctl show -p MainPID --value "$UNIT"; else cat "$PIDFILE"; fi
}

# Agent and login sessions each own a cgroup scope, and stopping the scope kills everything in it,
# so nohup and setsid are not enough. As root, run the server in its own system unit instead.
launch() {
  if use_systemd; then
    systemctl reset-failed "$UNIT" 2>/dev/null || true
    systemd-run --quiet --collect --unit="$UNIT" --working-directory="$PWD" \
      --setenv=PORT="$PORT" --setenv=PATH="$PATH" --setenv=HOME="$HOME" \
      -p Restart=on-failure -p RestartSec=3 \
      -p StandardOutput="append:$LOG" -p StandardError="append:$LOG" \
      pnpm exec "$@"
    rm -f "$PIDFILE"
  else
    PORT="$PORT" setsid nohup pnpm exec "$@" >> "$LOG" 2>&1 < /dev/null &
    echo $! > "$PIDFILE"
  fi
}

start() {
  if running; then echo "already running ($(pid)) on :$PORT"; return; fi
  if ss -ltn 2>/dev/null | grep -q ":$PORT "; then echo "port $PORT is in use by another process:"; ss -ltnp | grep ":$PORT "; exit 1; fi
  if [ "${1:-}" = "--preview" ]; then
    echo "building production bundle…"
    pnpm run build > "$LOG" 2>&1 || { echo "build failed"; tail -30 "$LOG"; exit 1; }
    launch vite preview
  else
    : > "$LOG"
    launch vite
  fi
  for _ in $(seq 1 60); do
    curl -s -o /dev/null "http://127.0.0.1:$PORT/" && { echo "live on :$PORT ($(pid))"; return; }
    sleep 0.5
  done
  echo "failed to start"; tail -30 "$LOG"; stop > /dev/null; exit 1
}

stop() {
  # Stop the unit first: killing its process would count as a failure and trigger a restart.
  if use_systemd; then systemctl stop "$UNIT" 2>/dev/null || true; fi
  if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then kill "$(cat "$PIDFILE")" 2>/dev/null || true; sleep 1; fi
  # Vite forks; make sure nothing is left holding the port.
  for p in $(ss -ltnp 2>/dev/null | grep ":$PORT " | grep -o 'pid=[0-9]*' | cut -d= -f2); do kill "$p" 2>/dev/null || true; done
  rm -f "$PIDFILE"; echo stopped
}

case "${1:-status}" in
  start) start "${2:-}" ;;
  stop) stop ;;
  restart) stop; start "${2:-}" ;;
  status) if running; then echo "running ($(pid)) on :$PORT"; else echo "not running"; fi ;;
  logs) tail -f "$LOG" ;;
  *) echo "usage: $0 start [--preview] | stop | restart [--preview] | status | logs"; exit 1 ;;
esac
