#!/usr/bin/env bash
set -u

root="$(pwd)"
status=0

while IFS= read -r -d '' subdir; do
  dir="$(dirname "$subdir")"
  echo "==> cargo deny in $dir"

  (
    cd "$dir" || exit 1
    cargo deny --version
    cargo deny check licenses --config "$root/deny.toml"
  ) || status=1
done < <(find . -mindepth 2 -name Cargo.toml -not -path '*/target/*' -print0)

exit "$status"
