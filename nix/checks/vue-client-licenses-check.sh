#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

# Licenses allowed via Apache list Category A
# This is used for a full string match of the license
allowedA=(
    "MIT"
    "MIT-0"
    "Apache-2.0"
    "BSD"
    "BSD-2-Clause"
    "BSD-3-Clause"
    "ISC"
    "BlueOak-1.0.0"
    "Python-2.0"
    "0BSD"
    "CC0-1.0"
)

# Packages allowed via Apache list Category B
# Supports regex for given package names
# lightningcss*: MPL-2.0
allowedB=(
    'lightningcss(-.*)?'
)

# Packages with licenses not listed by Apache, but similar to Category B
# Notice inside NOTICE.md file necessary
# Supports regex for given package names
# caniuse-lite: CC-BY-4.0
allowedX=(
    'caniuse-lite'
)

# Path to notice file
notice_file=${1:?Usage: vue-client-licenses-check NOTICE.md}
if [[ ! -f $notice_file ]]; then
    echo "NOTICE.md file does not exist: $notice_file" >&2
    exit 1
fi

# Verify a package is mentioned inside the notice file
notice_contains_package() {
    local package_name=$1

    # Expected NOTICE.md format: `## <package-name>`
    grep -Fqx -- "## $package_name" "$notice_file"
}

failed=0

raw_licenses="$(pnpm licenses list --json)"
license_list="$(
    jq -er 'to_entries[] | .key as $license | .value[] | [ $license, .name, (.versions | join(",")) ] | @tsv' \
        <<<"$raw_licenses"
)"

while IFS=$'\t' read -r license name versions; do
    # No issues with these licenses, can skip
    for allowed in "${allowedA[@]}"; do
        [[ $license == "$allowed" ]] && continue 2
    done

    # At least mention category B packages to keep an eye on them
    for allowed in "${allowedB[@]}"; do
        if [[ $name =~ $allowed ]]; then
            printf \
                'Allowed under Apache Category B: %s@%s with license %s\n' \
                "$name" "$versions" "$license"
            continue 2
        fi
    done

    # Verify that packages outside of the Apache catalogue have a third party notice
    # Fail if not
    for allowed in "${allowedX[@]}"; do
        if [[ $name =~ $allowed ]]; then
            printf \
                'Allowed with third-party notice: %s@%s with license %s\n' \
                "$name" "$versions" "$license"
            if notice_contains_package "$name"; then
                printf "✔ Notice exists.\n"
            else
                printf \
                    '✗ Missing NOTICE.md section: expected "## %s"\n' \
                    "$name" >&2
                failed=1
            fi
            continue 2
        fi
    done

    # Fail on any other license
    printf \
        'Disallowed package or license: %s in %s@%s\n' \
        "$license" "$name" "$versions" >&2
    failed=1
done <<<"$license_list"

if ((failed != 0)); then
    echo "Vue client license check failed." >&2
    exit 1
fi

echo "Vue client license check passed."
