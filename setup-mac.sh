#!/bin/sh
# One-time setup for Pintos on Apple Silicon (and Intel) macOS.
# Usage: ./setup-mac.sh   then   source env.sh   (or add it to ~/.zshrc)
set -e
cd "$(dirname "$0")"
command -v brew >/dev/null || { echo "Install Homebrew first: https://brew.sh"; exit 1; }
brew list i686-elf-gcc >/dev/null 2>&1 || brew install i686-elf-gcc
brew list i686-elf-binutils >/dev/null 2>&1 || brew install i686-elf-binutils
brew list qemu >/dev/null 2>&1 || brew install qemu
brew list i386-elf-gdb >/dev/null 2>&1 || brew install i386-elf-gdb || echo "(gdb optional)"
make -C utils
echo "Done. Run: source env.sh && cd threads && make check"
