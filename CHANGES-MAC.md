# Changes from the original Pintos source (macOS / Apple Silicon port)

Only build/tooling files were touched. No kernel, library or test source
(`threads/*.c`, `devices/`, `lib/`, `userprog/*.c`, `tests/` …) was modified.

## Modified files

| File | Change | Why |
|---|---|---|
| `Make.config` | Tool selection block replaced by `include Make.tools` | Single place to configure tools (see below) |
| `Makefile.userprog` | `libc.a` built with `$(AR)`/`$(RANLIB)` instead of bare `ar`/`ranlib` | macOS `ar` can't index ELF objects, so user programs failed to link |
| `threads/Make.vars`, `userprog/Make.vars`, `filesys/Make.vars`, `vm/Make.vars` | Removed the `SIMULATOR = --bochs` line | Simulator is now set centrally in `Make.tools` (default `--qemu`) |
| `utils/pintos` | (1) default simulator is bochs only if installed, else qemu; (2) default VGA is `window` on macOS (was only when `$DISPLAY` set); (3) skip `setpgrp(0,0)` when stdin is a tty; (4) qemu binary taken from `$PINTOS_QEMU` | (1) no Bochs; (2) macOS has no `$DISPLAY`, so no window appeared; (3) QEMU was put in a background process group and macOS froze it on tty access |
| `utils/pintos-gdb` | `GDBMACROS` points to `misc/gdb-macros` in this repo | Original path was a hardcoded university server path |
| `utils/Makefile` | `all:` builds `setitimer-helper` only | `squish-pty.c`/`squish-unix.c` need `stropts.h`, absent on macOS (only needed for Bochs terminal / serial piping) |

## Added files

- `Make.tools` — central toolchain/simulator definition (CC, LD, OBJCOPY, AR, RANLIB, SIMULATOR, PINTOS_QEMU) with Linux, macOS and Windows (MSYS2/Cygwin/WSL) detection. Override order: command line, environment, `Make.tools.local`, defaults.
- `Make.tools.local.example` — template for personal overrides (`Make.tools.local` is git-ignored).
- `setup-mac.sh` — one-time Homebrew install (`i686-elf-gcc`, `i686-elf-binutils`, `qemu`, optional `i386-elf-gdb`) and `utils` build.
- `env.sh` — `source` it to put `utils/` on `PATH`.
- `BUILD-MAC.md` — build/run guide per project, troubleshooting.
- `CHANGES-MAC.md` — this file.
- `.gitignore` — ignores `*/build/` and compiled helper binaries.

## Verified

`make check` for threads and userprog runs under QEMU (threads: 6/18 pass on
the unmodified skeleton; userprog: 0/80, as expected). vm and filesys build;
their `make check` runs and GDB use were not tried.
