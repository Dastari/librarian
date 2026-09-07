#!/usr/bin/env bash
# Start/stop the web dev server for screenshot runs. Usage: e2e/dev.sh start|stop|status
set -euo pipefail
cd "$(dirname "$0")/.."
PIDFILE=/tmp/librarian-web-dev.pid
case "${1:-status}" in
  start)
    if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then echo "already running ($(cat "$PIDFILE"))"; exit 0; fi
    nohup pnpm exec vite --port 3002 --host 0.0.0.0 > /tmp/web-dev.log 2>&1 &
    echo $! > "$PIDFILE"
    for _ in $(seq 1 30); do curl -s -o /dev/null http://127.0.0.1:3002/ && { echo "started ($(cat "$PIDFILE"))"; exit 0; }; sleep 0.5; done
    echo "failed to start"; tail -20 /tmp/web-dev.log; exit 1 ;;
  stop)
    if [ -f "$PIDFILE" ]; then kill "$(cat "$PIDFILE")" 2>/dev/null || true; rm -f "$PIDFILE"; echo stopped; fi ;;
  status)
    if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then echo "running ($(cat "$PIDFILE"))"; else echo "not running"; fi ;;
esac
