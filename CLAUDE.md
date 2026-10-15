# CLAUDE.md

Guidance for Claude Code working in this repo.

## What this is

`neopintos` is the Pintos teaching OS (32-bit x86, UT CS439-style skeleton) with two additions:

1. A macOS/Apple Silicon build port (`i686-elf-*` cross toolchain + QEMU). See `BUILD-MAC.md`, `CHANGES-MAC.md`.
2. An incremental introduction of Rust into the kernel, in `rust/`. Work in progress.

The kernel, lib, devices and tests are the stock skeleton. Most projects are **intentionally unimplemented** (students fill them in), so most stock tests fail by design. Do not "fix" that by editing or weakening tests.

## Layout

`threads/ userprog/ vm/ filesys/` = projects 1-4 (each has `Make.vars`, builds in `<proj>/build/`). `devices/ lib/` kernel + libc. `tests/` suites and `.ck` checkers. `examples/` user programs. `utils/` `pintos` launcher and disk tools (Python 3 + `pintos_common.py`; Perl originals in `old_pintos_scripts/`; `tests/` checkers are still Perl). `rust/` Rust crate, baselines, check script. `Make.tools` = the one place toolchain vars live (`CC LD OBJCOPY AR RANLIB SIMULATOR PINTOS_QEMU USE_RUST CARGO`).

## Building and testing

```sh
export PATH=$PWD/utils:$HOME/.cargo/bin:$PATH
cd threads && make            # creates build/ subdirs; see cowsay gotcha below
cd build && make -j8 && make check -j4
```

- **Cowsay gag:** `Makefile.kernel`'s top-level `all:` randomly (1 in 3) prints cowsay and exits 1. Not a real error. After the first `make` has created `build/`, run `make` inside `build/` directly.
- **filesys tests share one `tmp.dsk`:** run `make -k check -j1` there, never in parallel.
- `utils/`: only `setitimer-helper` needs building (`make -C utils setitimer-helper`); `squish-*` fail on missing `stropts.h` and are not needed.
- `Make.tools` currently holds the user's local Linux settings (`gcc -m32`, `ld -melf_i386`); the macOS block is commented out. Do not revert that edit.
- Run a kernel by hand: `cd threads/build && pintos --qemu -v -k -T 60 -- -q run alarm-single`. For FILESYS builds add `--filesys-size=2` and `-f`; vm also needs `--swap-size=4`.

## Rust integration (`rust/`)

- `USE_RUST` (Make.tools, default 1) links `rust/` as a `no_std` staticlib into `kernel.o` (`Makefile.build`, `$(RUST_LIB)`, built into `<proj>/build/rust-target`). `USE_RUST=0` builds the original all-C kernel. After flipping it, `make clean` first; objects depend on `-DRUST`.
- Toolchain: nightly pinned in `rust/rust-toolchain.toml` (`nightly-2026-08-01`, with `rust-src`), custom target `rust/i686-pintos.json`, `-Zbuild-std=core,alloc` via `rust/.cargo/config.toml`. `cargo` must be on `PATH`. Makefile runs cargo with `env -u MAKEFLAGS` to avoid a jobserver warning.
- Per-project Cargo features come from `RUST_FEATURES` in each `<proj>/Make.vars` (`userprog`, `filesys`, `vm`), mirroring the `-D` flags. Rust code calling a C symbol that only exists in some projects must be behind the matching feature.
- Crate layout (`rust/src/`): `sys.rs` (FFI declarations), `rt.rs` (panic handler -> `debug_panic`, `#[global_allocator]` over kernel `malloc`, `kprintln!`), `selftest.rs`. Ported subsystems live beside their C originals and are included via `#[path]` in `lib.rs`: `lib/kernel/bitmap.rs`, `lib/kernel/list.rs`. `rust_init()` is called from `threads/init.c` after `malloc_init()` and must stay silent (stock boot output must not change). Action `rust-selftest` runs in-kernel checks.
- Ported components keep the exact C ABI/struct layout (e.g. `struct bitmap`), and the C source is excluded from the build when `USE_RUST=1` (see `lib/kernel_SRC` in `Makefile.build`).

Gotchas learned the hard way:

- **Linker script:** Rust emits per-symbol sections. `threads/kernel.lds.S` matches `.text.*`, `.data.*`, `.bss.*` so statics land inside `[_start_bss, _end_bss)`, which `bss_init()` zeroes. Don't remove those patterns.
- `kernel` `malloc` only guarantees 4-byte alignment; the Rust allocator over-allocates for larger alignments. Never allocate from interrupt context.
- Don't enable compiler_builtins' `mem` feature: `lib/string.c` already provides `memcpy` etc., and `lib/arithmetic.c` provides the 64-bit division helpers.
- Kernel stacks are 4 KB shared with `struct thread`: keep Rust frames small, release profile only.
- Don't grep build output for `error`: every compile line contains `-Werror`.

## Verifying changes (the contract)

Every milestone must boot and match stock behavior. Use:

```sh
rust/check.sh                  # all four projects, USE_RUST default (1); also runs rust-selftest
USE_RUST=0 rust/check.sh       # the all-C kernel
rust/check.sh threads          # one project
```

It rebuilds clean, runs `make check`, and diffs pass/FAIL per test against `rust/baseline/<proj>.txt`. Baseline (stock C): threads 5/18, userprog 0/80, vm 0/94, filesys 0/125. `rust/baseline/flaky.txt` lists tests nondeterministic in the all-C kernel (currently `alarm-priority`); only those may differ. `--record` rewrites baselines: only do that from a `USE_RUST=0` stock build, and only if asked.

## Status and next steps

Done: Rust runtime, Rust bitmap, self-test, baseline harness. Not done / planned:

- **`fillgaps/`:** minimal student-implemented pieces needed to validate userprog Rust code (arg passing, `exec`/`wait`/`exit`, syscalls, user-pointer validation). Plan: mirror the repo tree under `fillgaps/`, shadow stock files via `vpath` (and `-I fillgaps`) only when `FILLGAPS=1`, keep Rust gap modules in `fillgaps/rust/` behind a `fillgaps` feature, and keep default builds identical to stock. Only fill what is needed to exercise the Rust code (skip e.g. rox tests, scheduler, vm).
- Port `list.c` (and maybe `hash.c`) with the same ABI.
- `setup-rust.sh` is referenced by `Make.tools` comments but does not exist yet.
- vm/filesys only verified to boot and match the all-fail baseline; Rust bitmap file read/write paths are untested.
- Nothing from the Rust work is committed yet.

## Conventions

- Don't edit tests or `.ck` files. Mark anything a student would normally write under `fillgaps/`.
- Keep stock files' diffs minimal; build-system changes are fine.
- `unsafe` in Rust only at hardware/FFI edges, with `// SAFETY:` comments (`#![deny(unsafe_op_in_unsafe_fn)]` is on).
- Commit only when asked. Commit messages end with the attribution line from the session's system reminder.
