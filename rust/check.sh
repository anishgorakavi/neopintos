#!/bin/sh
# Build each Pintos project and run its stock `make check`, then compare the
# pass/FAIL outcome of every test with the recorded stock-C baseline in
# rust/baseline/.  Tests are never edited or skipped; any difference is an error,
# except tests listed in rust/baseline/flaky.txt (nondeterministic in the all-C kernel).
# For Rust builds it also runs the in-kernel `rust-selftest` action.
#
# usage: [USE_RUST=0|1] rust/check.sh [--record] [project...]
#        (default projects: threads userprog vm filesys)
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
export PATH="$root/utils:$HOME/.cargo/bin:$PATH"
# USE_RUST=0 builds the original all-C kernel; default (1) links the Rust components.
export USE_RUST=${USE_RUST:-1}
record=0
[ "${1:-}" = "--record" ] && { record=1; shift; }
projects=${*:-threads userprog vm filesys}
status=0

# Drop flaky tests from a result list.
unflaky() { grep -vFf "$tmp.flaky" "$1"; }
tmp=$(mktemp)
grep -vE '^#|^$' "$root/rust/baseline/flaky.txt" | sed 's/^/ /' > "$tmp.flaky"

for p in $projects; do
    jobs=-j4; [ "$p" = filesys ] && jobs=-j1   # filesys tests share one tmp.dsk
    # The top-level make creates build/ and its subdirectories; its random "cowsay"
    # gag may abort it, so run the real build directly in build/ afterwards.
    ( cd "$root/$p" && { make >/dev/null 2>&1; true; } && cd build \
        && make clean >/dev/null 2>&1 && make -j8 >"$tmp.build" 2>&1; make -k check $jobs >"$tmp.out" 2>&1 )
    grep -E '^(pass|FAIL) ' "$tmp.out" | sort -u > "$tmp.res"
    if [ ! -s "$tmp.res" ]; then
        echo "$p: no test results (build failed?)"; tail -5 "$tmp.build"; status=1; continue
    fi
    if [ $record = 1 ]; then
        cp "$tmp.res" "$root/rust/baseline/$p.txt"
        echo "$p: recorded $(grep -c '^pass' "$tmp.res") pass / $(wc -l < "$tmp.res") total"
        continue
    fi
    if [ "$USE_RUST" = 1 ]; then
        case $p in
            threads) disk=""; kargs="-q" ;;
            vm)      disk="--filesys-size=2 --swap-size=4"; kargs="-q -f" ;;
            *)       disk="--filesys-size=2"; kargs="-q -f" ;;
        esac
        st=$(cd "$root/$p/build" && pintos --qemu -v -k -T 60 $disk -- $kargs rust-selftest 2>&1 | grep -E '^\(rust-selftest\)')
        if echo "$st" | grep -q 'end: [1-9][0-9]* checks passed'; then
            echo "$p: $(echo "$st" | grep 'end:')"
        else
            echo "$p: rust-selftest FAILED"; echo "$st"; status=1
        fi
    fi
    unflaky "$root/rust/baseline/$p.txt" > "$tmp.base.f"; unflaky "$tmp.res" > "$tmp.res.f"
    if diff -u "$tmp.base.f" "$tmp.res.f"; then
        echo "$p: OK, identical to baseline ($(grep -c '^pass' "$tmp.res") pass / $(wc -l < "$tmp.res") total)"
    else
        echo "$p: DIFFERS FROM BASELINE"; status=1
    fi
done
rm -f "$tmp" "$tmp".*
exit $status
