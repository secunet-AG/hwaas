#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

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
