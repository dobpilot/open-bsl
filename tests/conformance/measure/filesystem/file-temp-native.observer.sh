#!/usr/bin/env bash
set -euo pipefail
work=/tmp/open-bsl-temp-native-approved-8vaLq3
pid=1416828
test -d "/proc/$pid/fd"
deadline=$((SECONDS + 115))
for phase in a0 a1 a2 a3 a4 a5 b0 b1 b2 b3 b4 b5 c0 c1 c2 c3 c4 c5; do
    until test -s "$work/platform.tsv" && rg -q "^native.phase[[:space:]]+$phase" "$work/platform.tsv"; do
        if (( SECONDS > deadline )); then
            printf 'observer.timeout %s\n' "$phase"
            exit 1
        fi
        sleep 0.05
    done
    printf 'phase %s\n' "$phase"
    for fd in /proc/$pid/fd/*; do
        target=$(readlink "$fd") || continue
        case "$target" in
            /tmp/v8_*.tmp|/tmp/v8_*.tmp\ \(deleted\))
                metadata=$(stat -L -c '%i %s' "$fd")
                printf 'fd %s %s %s\n' "${fd##*/}" "$metadata" "$target"
                ;;
        esac
    done
    touch "$work/ack-$phase"
done
printf 'observer.complete\n'
