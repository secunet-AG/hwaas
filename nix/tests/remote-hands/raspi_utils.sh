#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# This file contains helper functions for the remote-hands integration tests.

# Removes all ansi command sequences from the stdin string
function clearAnsi() {
    sed -e "s,\x1B\[[0-9;]*[a-zA-Z],,g" -e "s,\x1b]0;.*\x07,,g" -e 's,\x1b\[[0-9?]\+[^m0-9?],,g'
}

# Wait for the command prompt of the OS. This function fails if the time needed for the command
# prompt to occur is too long.
function waitForCommandPrompt() {
    host=$1

    ansiOverrideCurrentLine='\r'
    bashLogin="[nixos@nixos:~]$ "
    start_time="$(date -u +%s)"
    while true; do
        serial=$(http -I --check-status GET "$host/serial/tty" | tr -d '\0')
        echo "$serial"
        # remove ansi command sequences
        lastLineCleaned=$(echo "$serial" | tail -n1 | clearAnsi)

        [[ $lastLineCleaned != "$bashLogin" ]] || break

        sleep 1s
        current_time="$(date -u +%s)"
        printf "%bWaiting for command prompt ... ($((current_time - start_time))s)" $ansiOverrideCurrentLine

        duration=$((current_time - start_time))
        if [ $duration -gt 90 ]; then
            printf "\nFailed waiting for command prompt: Timeout\n"
            exit 1
        fi
    done
    printf "\n"
}

# Assert that the 1st argument is equal to the 2nd argument.
function assertEqual() {
    actual=$1
    expected=$2

    if [[ $expected != "$actual" ]]; then
        echo "Assertion failed: Output was '$actual' but expected '$expected'."
        exit 1
    fi
}

# Assert that the 1st argument string contains the 2nd argument string.
function assertContains() {
    output=$1
    contained=$2

    if [[ $output != *"$contained"* ]]; then
        echo "Assertion failed: Output was '$output' but expected it to contain '$contained'."
        exit 1
    fi
}
