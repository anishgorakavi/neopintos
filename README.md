# neopintos

Pintos, the Stanford/UT teaching operating system for 32-bit x86, with the build
tooling ported so it builds and runs on **macOS (Apple Silicon or Intel)**
using an `i686-elf` cross-compiler and **QEMU**. No Bochs and no x86 Linux
host are required.

The kernel, library, device, and test sources are the original Pintos skeleton.
Only build and tooling files differ (see [Changes from original Pintos](#changes-from-original-pintos)).

## Quick start

```sh
./setup-mac.sh     # brew installs i686-elf-gcc, i686-elf-binutils, qemu (and optional i386-elf-gdb); builds utils/
source env.sh      # puts utils/ on PATH and defaults to QEMU
cd threads && make -j8 && make check
```

Run commands from the repo root after `source env.sh`. Add the `source` line to
`~/.zshrc` to make it permanent.

## Repository layout

| Directory | Contents |
|---|---|
| `threads/` | Project 1: scheduler, synchronization, timer alarm (`README.threads`) |
| `userprog/` | Project 2: user programs, syscalls, argument passing (`README.userprog`) |
| `vm/` | Project 3: virtual memory (`README.vm`) |
| `filesys/` | Project 4: file system (`README.filesys`) |
| `devices/`, `lib/` | Device drivers and the kernel/user C library |
| `tests/` | Test suites and the grading scripts for each project |
| `examples/` | Sample user programs (`cat`, `ls`, `shell`, ...) |
| `utils/` | `pintos` launcher, `pintos-gdb`, `backtrace`, disk helpers |
| `misc/` | Bochs patches/build scripts, GDB macros, GCC cross-compiler notes |
| `Make.tools` | Toolchain definition (see below) |

## Choosing a toolchain

All tools are named in one file, **`Make.tools`**: `CC`, `LD`, `OBJCOPY`, `AR`,
`RANLIB`, `SIMULATOR`, and `PINTOS_QEMU`. It ships set for macOS
(`i686-elf-*`, `--qemu`). It also has a commented block for native x86-64 Linux
(`gcc -m32`); uncomment it and comment out the macOS block to use it. Only
macOS has been run. Windows would need MSYS2 or Cygwin with an ELF cross
toolchain.

## Building and testing

Each project is built from its own directory:

```sh
cd threads  && make -j8 && make check     # also: make grade
cd userprog && make -j8 && make check
cd vm       && make -j8 && make check
cd filesys  && make -j8 && make check
make tests/threads/alarm-single.result    # a single test, from inside a project dir
```

Run a kernel by hand:

```sh
cd threads/build && pintos -- -q run alarm-single
```

For the full per-project walkthrough (running user programs, GDB, flags) and a
troubleshooting table, see **[BUILD-MAC.md](BUILD-MAC.md)**.

Status of the unmodified skeleton under QEMU: `threads` passes 6 of 18 tests
(the `alarm-*` tests that need no student code) and `userprog` passes 0 of 80
until syscalls and argument passing exist. `vm` and `filesys` build, but their
`make check` runs and GDB use were not tried.

## Changes from original Pintos

Only build and tooling files were touched. No kernel, library, or test source
was modified. Full details are in **[CHANGES-MAC.md](CHANGES-MAC.md)**.

**Modified**

- `Make.config`: tool selection replaced by `include Make.tools`.
- `Makefile.userprog`: `libc.a` is built with `$(AR)`/`$(RANLIB)`, because macOS `ar` cannot index ELF objects and user programs failed to link.
- `*/Make.vars` (threads, userprog, filesys, vm): removed `SIMULATOR = --bochs`; the simulator is set in `Make.tools` (default `--qemu`).
- `utils/pintos`:
  - Defaults to Bochs only if it is installed, otherwise QEMU.
  - Opens a QEMU window on macOS, which has no `$DISPLAY`.
  - Skips `setpgrp(0,0)` when stdin is a tty, so macOS no longer freezes QEMU on tty access.
  - Reads the QEMU binary from `$PINTOS_QEMU`.
- `utils/pintos-gdb`: `GDBMACROS` points at `misc/gdb-macros` in this repo instead of a hardcoded university server path.
- `utils/Makefile`: builds only `setitimer-helper`, because `squish-pty`/`squish-unix` need `stropts.h`, which macOS lacks. They only matter for the Bochs terminal and serial piping.

**Added**

- `Make.tools`: the single toolchain definition.
- `setup-mac.sh`: one-time Homebrew setup.
- `env.sh`: `PATH` and simulator setup.
- `BUILD-MAC.md`, `CHANGES-MAC.md`: build guide and change log.
- `.gitignore`: ignores `*/build/` and compiled helper binaries.

**History**

1. Port build tooling to macOS with `i686-elf-gcc` and QEMU.
2. Add `Make.tools` as the central toolchain definition (with per-OS detection).
3. Simplify `Make.tools` to a plain list of assignments; the developer edits it directly.

## Rust components (work in progress)

Rust is being introduced incrementally behind the original C ABI. `USE_RUST` in `Make.tools` (default 1)
links the `no_std` static library in `rust/` into `kernel.o`; `USE_RUST=0` builds the original all-C kernel.

- Toolchain: nightly pinned in `rust/rust-toolchain.toml` (needs `rustup` with `rust-src`; `cargo` must be on `PATH`).
  Target is the custom `rust/i686-pintos.json`; the Makefile runs cargo with `-Zbuild-std`.
- Milestone 0/1: Rust runtime (panic handler -> Pintos `PANIC`, allocator over kernel `malloc`, `kprintln!`).
- Milestone 2: `lib/kernel/bitmap.c` replaced by `rust/src/bitmap.rs` (same C ABI, bounds-checked).
- Milestone 3: `lib/kernel/list.c` replaced by `rust/src/list.rs` (same C ABI and struct layout; the C `ASSERT`s are kept as `assert!`s).
- `threads/kernel.lds.S` now also matches `.text.*`, `.data.*`, `.bss.*` so Rust's per-symbol sections land inside the zeroed BSS.
- `pintos -- -q rust-selftest` runs the in-kernel Rust self-tests.
- `rust/check.sh [--record] [project...]` runs stock `make check` for threads/userprog/vm/filesys and diffs pass/FAIL against
  `rust/baseline/` (recorded from the stock C kernel). `rust/baseline/flaky.txt` lists tests that vary in the all-C kernel.
- Planned (not started): `fillgaps/` holding the minimal student-implemented userprog pieces (syscalls, arg passing, wait) in Rust.

### Building and running the Rust kernel

1. **Install the toolchain (once).** The kernel needs a nightly Rust with `rust-src` (for `-Zbuild-std`).
   Install [rustup](https://rustup.rs); inside the repo it then picks up the pinned nightly from
   `rust/rust-toolchain.toml` automatically and fetches `rust-src` on first use.
   Also run `./setup-mac.sh` if you have not yet installed `i686-elf-gcc` and QEMU.
2. **Set up the shell** (from the repo root):

   ```sh
   source env.sh
   export PATH=$HOME/.cargo/bin:$PATH     # so `cargo` is found
   ```

3. **Build.** `USE_RUST` defaults to 1, so a normal build already links the Rust code:

   ```sh
   cd threads && make                     # first time only: creates build/ (may print the cowsay gag and exit 1; just continue)
   cd build && make -j8
   ```

   The Rust library is built into `<project>/build/rust-target/`. Build any other project (`userprog`, `vm`, `filesys`)
   the same way. To build the original all-C kernel instead, run `make clean` and rebuild with `USE_RUST=0`
   (for example `USE_RUST=0 make -j8`); always `make clean` after changing `USE_RUST`.
4. **Run the Rust self-tests** inside the kernel (prints one `(rust-selftest) ok: ...` line per check):

   ```sh
   cd threads/build && pintos --qemu -v -k -T 60 -- -q rust-selftest
   ```

   For `filesys`/`vm` builds add `--filesys-size=2` and `-f` (and `--swap-size=4` for `vm`), as for any other run.
5. **Check against stock behavior.** From the repo root:

   ```sh
   rust/check.sh                  # all four projects with Rust, plus rust-selftest
   USE_RUST=0 rust/check.sh       # the all-C kernel
   rust/check.sh threads          # one project
   ```

   Do not use `--record` unless you intend to rewrite the baselines (and only from a `USE_RUST=0` build).

If you have no rustup, Homebrew's stable `rust` (which includes `rust-src`) can build the kernel when you set
`RUSTC_BOOTSTRAP=1` in the environment (`RUSTC_BOOTSTRAP=1 rust/check.sh`). This is a workaround, not the pinned
toolchain, so results may differ from the intended nightly.

Notes for writing kernel Rust: kernel stacks are 4 KB, so avoid large stack arrays and std routines that use them
(for example `slice::sort`, whose stable sort keeps a 4 KB scratch buffer on the stack).

## License

See [LICENSE](LICENSE).
