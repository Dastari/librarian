#!/usr/bin/env bash
# Sandbox backend for screenshots and manual QA: a copy of the real database and artwork under
# /tmp/librarian-sandbox, served on :3011, plus a dev server on :3003 that proxies to it.
# Scans, organization and DHT are disabled in the copy so real media is never touched.
#
#   e2e/sandbox.sh start | stop | status | reset
#
# Sign in with toby / sandbox-password-123 (exists only in the sandbox copy).
set -euo pipefail
cd "$(dirname "$0")/.."
SB=/tmp/librarian-sandbox
REPO=$(cd .. && pwd)

start_backend() {
  if [ -f "$SB/backend.pid" ] && kill -0 "$(cat "$SB/backend.pid")" 2>/dev/null; then echo "backend running"; return; fi
  ( cd "$SB" && set -a && . "$REPO/backend/.env" && set +a \
    && PORT=3011 HOST=127.0.0.1 DATABASE_PATH=$SB/data/librarian.db MEDIA_PATH=$SB/data/media DOWNLOADS_PATH=$SB/data/downloads \
       CACHE_PATH=$SB/data/cache SESSION_PATH=$SB/data/session STORAGE_PATH=$SB/data/storage BACKUP_PATH=$SB/data/backups \
       LIBRARIAN_CORS_ORIGINS="http://localhost:3003,http://127.0.0.1:3003" LIBRARIAN_SECURE_COOKIES=false TORRENT_LISTEN_PORT=6899 \
       nohup "$REPO/backend/target/debug/librarian" --server > "$SB/backend.log" 2>&1 & echo $! > "$SB/backend.pid" )
  for _ in $(seq 1 40); do curl -s -o /dev/null http://127.0.0.1:3011/graphql -H 'content-type: application/json' -d '{"query":"{ needsSetup }"}' && { echo "backend started"; return; }; sleep 0.5; done
  echo "backend failed"; tail -20 "$SB/backend.log"; exit 1
}

start_web() {
  if [ -f /tmp/librarian-web-shots.pid ] && kill -0 "$(cat /tmp/librarian-web-shots.pid)" 2>/dev/null; then echo "web running"; return; fi
  BACKEND_PROXY_TARGET=http://127.0.0.1:3011 nohup pnpm exec vite --port 3003 --host 127.0.0.1 > /tmp/web-shots.log 2>&1 &
  echo $! > /tmp/librarian-web-shots.pid
  for _ in $(seq 1 30); do curl -s -o /dev/null http://127.0.0.1:3003/ && { echo "web started on :3003"; return; }; sleep 0.5; done
  echo "web failed"; tail -20 /tmp/web-shots.log; exit 1
}

case "${1:-status}" in
  start) [ -d "$SB/data" ] || { echo "no sandbox data; run: e2e/sandbox.sh reset"; exit 1; }; start_backend; start_web ;;
  stop)
    for pid in "$SB/backend.pid" /tmp/librarian-web-shots.pid; do
      [ -f "$pid" ] && { kill "$(cat "$pid")" 2>/dev/null || true; rm -f "$pid"; }
    done; echo stopped ;;
  status)
    for name in backend web; do
      pid=$([ "$name" = backend ] && echo "$SB/backend.pid" || echo /tmp/librarian-web-shots.pid)
      if [ -f "$pid" ] && kill -0 "$(cat "$pid")" 2>/dev/null; then echo "$name running ($(cat "$pid"))"; else echo "$name not running"; fi
    done ;;
  reset)
    "$0" stop
    rm -rf "$SB" && mkdir -p "$SB"/data/{downloads,cache,session,backups,media}
    python3 - <<'PY'
import sqlite3
src=sqlite3.connect('file:/root/librarian/backend/data/librarian.db?mode=ro', uri=True)
dst=sqlite3.connect('/tmp/librarian-sandbox/data/librarian.db')
src.backup(dst)
dst.execute("delete from refresh_tokens"); dst.execute("delete from users")
dst.execute("update libraries set auto_scan=0, auto_organize=0, watch_for_changes=0, scanning=0")
dst.execute("update app_settings set value='\"/tmp/librarian-sandbox/data/downloads\"' where key='torrent.download_dir'")
dst.execute("update app_settings set value='\"/tmp/librarian-sandbox/data/session\"' where key='torrent.session_dir'")
dst.execute("update app_settings set value='false' where key='torrent.enable_dht'")
dst.execute("update app_settings set value='6899' where key='torrent.listen_port'")
dst.commit()
print("sandbox database ready; complete setup at http://127.0.0.1:3003/register then run: e2e/sandbox.sh adopt")
PY
    cp -r "$REPO/backend/data/storage" "$SB/data/storage"
    start_backend; start_web ;;
  adopt)
    python3 - <<'PY'
import sqlite3
c=sqlite3.connect('/tmp/librarian-sandbox/data/librarian.db')
users=c.execute("select id from users").fetchall()
assert users, "register the sandbox admin first"
uid=users[0][0]
for (t,) in c.execute("select name from sqlite_master where type='table'").fetchall():
    cols=[r[1] for r in c.execute(f"pragma table_info({t})").fetchall()]
    if 'user_id' in cols: c.execute(f"update {t} set user_id=? where user_id!='system'", (uid,))
c.commit(); print("sandbox data now belongs to", uid)
PY
    ;;
esac
