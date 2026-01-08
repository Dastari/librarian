#!/usr/bin/env bash
set -euo pipefail

ARCH=amd64   # use arm64 if needed
curl -fsSL "https://github.com/supabase/cli/releases/latest/download/supabase_linux_${ARCH}.tar.gz" \
  | tar -xz supabase
sudo mv supabase /usr/local/bin/
supabase --version
