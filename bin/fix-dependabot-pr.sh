#!/usr/bin/env bash
#
# SPDX-FileCopyrightText: 2026 secunet Security Networks AG <https://www.secunet.com>
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

declare -A args=(
    [branch]=""
    [remote]="origin"
    [worktrees_subdir]="worktrees"
)
declare -r HWAAS_API_URL="https://api.github.com/repos/secunet-ag/hwaas"

function _fatal {
    for line in "$@"; do
        echo -e "\e[31mFATAL\e[0m  $line" 1>&2
    done
    exit 1
}

function _usage {
    cat <<CLIHELP
Usage: $(basename "$0") [OPTION...] BRANCH_OR_PR
Manually check out and fix dependabot PRs.

Available Options:
  -h, --help            Show this help text.
      --remote NAME     Search for the dependabot PR branch in this remote.
                        (default: ${args[remote]})
      --worktrees SUBDIR
                        Check out worktrees in this subdirectory of the project
                        root.
                        (default: ${args[worktrees_subdir]})

Positional Arguments:
  BRANCH_OR_PR          The branch name (starting with 'depedabot/') or numeric
                        PR ID (i.e. '#123' or just '123') of the dependabot PR
                        to check out.

This script checks out the given dependabot PR in a new branch on a local
worktree and launches an interactive bash shell. Within this shell, you can
modify and amend the changes performed by dependabot. Once the shell exits, the
created worktree and local working branch are removed. Make sure to persist
(i.e. push) your changes before leaving the shell.
CLIHELP
}

while [[ $# -gt 0 ]]; do
    case "$1" in
    "-h" | "--help")
        _usage
        exit 0
        ;;
    "--remote")
        [[ $# -ge 2 ]] || _fatal "'$1' needs value NAME"
        args[remote]="$2"
        shift 2
        ;;
    "--worktrees")
        [[ $# -ge 2 ]] || _fatal "'$1' needs value SUBDIR"
        args[worktrees_subdir]="$2"
        shift 2
        ;;
    -*)
        _fatal "unknown argument '$1'"
        ;;
    *)
        if [[ -z "${args[branch]:-}" ]]; then
            if [[ "$1" =~ ^dependabot/ ]]; then
                args[branch]="$1"
            elif [[ "$1" =~ ^#?([1-9][0-9]*)$ ]]; then
                mr_metadata="$(curl "$HWAAS_API_URL/pulls/${BASH_REMATCH[1]}")"
                if "$(jq -er '.merged' <<<"$mr_metadata")"; then
                    _fatal "cannot work on merged branches"
                fi
                args[branch]="$(jq -er '.head.ref' <<<"$mr_metadata")"
            else
                _fatal "cannot convert invalid argument '$1' to dependabot branch"
            fi
            shift
        else
            _fatal "expected exactly one positional argument, second was '$1'"
        fi
        ;;
    esac
done

# Blindly assume the branch lives on "origin"
dirty_project_root="$(git rev-parse --path-format=absolute --git-common-dir)/.."
project_root="$(realpath "$dirty_project_root")"
worktree_dir="$project_root/${args[worktrees_subdir]}"
mkdir -p "$worktree_dir"
git worktree add --track -b "${args[branch]}" \
    "$worktree_dir/${args[remote]}/${args[branch]}" "${args[remote]}/${args[branch]}"
pushd "$worktree_dir/${args[remote]}/${args[branch]}" &>/dev/null

echo -ne "\e[35m"
cat <<TEXT 1>&2
Launching a new shell with the checked out branch in:
    $worktree_dir/${args[remote]}/${args[branch]}
When you're done, just exit the shell to return to where you were before.
The branch will be deleted in the process, so make sure to commit your changes.
TEXT
echo -ne "\e[0m"

function _cleanup {
    git worktree remove -f "$worktree_dir/${args[remote]}/${args[branch]}"
    git branch -D "${args[branch]}"
}
trap _cleanup EXIT

bash -i
popd &>/dev/null
