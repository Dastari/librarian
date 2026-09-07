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

running() { [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; }

start() {
  if running; then echo "already running ($(cat "$PIDFILE")) on :$PORT"; return; fi
  if ss -ltn 2>/dev/null | grep -q ":$PORT "; then echo "port $PORT is in use by another process:"; ss -ltnp | grep ":$PORT "; exit 1; fi
  if [ "${1:-}" = "--preview" ]; then
    echo "building production bundle…"
    pnpm run build > "$LOG" 2>&1 || { echo "build failed"; tail -30 "$LOG"; exit 1; }
    PORT="$PORT" nohup pnpm exec vite preview >> "$LOG" 2>&1 &
  else
    PORT="$PORT" nohup pnpm exec vite > "$LOG" 2>&1 &
  fi
  echo $! > "$PIDFILE"
  for _ in $(seq 1 60); do
    curl -s -o /dev/null "http://127.0.0.1:$PORT/" && { echo "live on :$PORT ($(cat "$PIDFILE"))"; return; }
    sleep 0.5
  done
  echo "failed to start"; tail -30 "$LOG"; exit 1
}

stop() {
  if running; then kill "$(cat "$PIDFILE")" 2>/dev/null || true; sleep 1; fi
  # Vite forks; make sure nothing is left holding the port.
  for pid in $(ss -ltnp 2>/dev/null | grep ":$PORT " | grep -o 'pid=[0-9]*' | cut -d= -f2); do kill "$pid" 2>/dev/null || true; done
  rm -f "$PIDFILE"; echo stopped
}

case "${1:-status}" in
  start) start "${2:-}" ;;
  stop) stop ;;
  restart) stop; start "${2:-}" ;;
  status) if running; then echo "running ($(cat "$PIDFILE")) on :$PORT"; else echo "not running"; fi ;;
  logs) tail -f "$LOG" ;;
  *) echo "usage: $0 start [--preview] | stop | restart [--preview] | status | logs"; exit 1 ;;
esac
