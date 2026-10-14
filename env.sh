# source this file: puts pintos utils on PATH, defaults to QEMU.
_d="$(cd "$(dirname "${BASH_SOURCE[0]:-${(%):-%x}}")" && pwd)"
export PATH="$_d/utils:$PATH"
export SIMULATOR="--qemu"
unset _d
