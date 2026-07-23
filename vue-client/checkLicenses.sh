#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

# Licenses allowed via Apache list Category A
allowedA='^(MIT|MIT-0|Apache-2.0|BSD|BSD-2-Clause|BSD-3-Clause|ISC|BlueOak-1.0.0|Python-2.0|0BSD|CC0.1.0)$'
# Packages allowed via Apache list Category B
# lightningcss*: MPL-2.0
allowedB='^(lightningcss(-.*)?)$'
# Packages with licenses not listed, but similar to Category B
# caniuse-lite: CC-BY-4.0
allowedX='^(caniuse-lite)$'

json=$(
  pnpm licenses list --json |
    jq -r '
	to_entries[]
	| .key as $license
	| .value[]
	| [$license, .name, (.versions | join(","))] | @tsv
'
)

echo "$json" | while IFS=$'\t' read -r license name version; do
  if ! [[ $license =~ $allowedA ]]; then
    if [[ $name =~ $allowedB ]]; then
      echo "Allowed license under Apache Category B: $name@$version with license $license"
    elif [[ $name =~ $allowedX ]]; then
      echo "Allowed license with third party notice: $name@$version with license $license"
    else
      echo "Disallowed license: $license in $name@$version"
    fi
  fi
done
