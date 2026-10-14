# Building Pintos on macOS (Apple Silicon or Intel)

Pintos is a 32-bit x86 OS, so on an ARM Mac we use an `i686-elf` cross-compiler
and run the kernel under QEMU (software emulation; no Bochs needed).

## One-time setup

```sh
./setup-mac.sh        # brew installs i686-elf-gcc, i686-elf-binutils, qemu; builds utils/
source env.sh         # adds utils/ to PATH (add this line to ~/.zshrc to make it permanent)
```

`Make.config` auto-detects the cross tools (`i686-elf-gcc/ld/objcopy/ar/ranlib`),
`*/Make.vars` default to `--qemu`, and `utils/pintos` falls back to QEMU when
Bochs is absent. The `make check` flow below is verified for threads and
userprog; filesys and vm build, but their `make check` runs were not tried.

Run every command below from the repo root after `source env.sh`.

## Changing the toolchain

Edit **`Make.tools`**. It is a plain list of assignments (`CC`, `LD`, `OBJCOPY`,
`AR`, `RANLIB`, `SIMULATOR`, `PINTOS_QEMU`) and the only place tools are named.
It ships set for macOS, with a commented x86-64 Linux block to swap in. Only
macOS was run here. Windows needs MSYS2 or Cygwin with an ELF cross toolchain.

## Project 1 — threads

```sh
cd threads && make -j8                 # builds build/kernel.bin
make check                             # all tests (also: make grade)
make tests/threads/alarm-single.result # one test
cd build && pintos -- -q run alarm-single   # run a test by hand
```

Baseline skeleton: the six `alarm-*` tests pass, `priority-*` need your code.

## Project 2 — userprog

```sh
cd userprog && make -j8                # kernel + user-program tests
make check                             # all tests
make tests/userprog/args-none.result   # one test
```

Run a user program by hand (copy it into the simulated disk, then execute):

```sh
cd userprog/build
pintos --qemu -v -k -T 60 --filesys-size=2 \
  -p tests/userprog/args-none -a args-none -- -q -f run args-none
```

Flags: `-p <host file> -a <name in pintos fs>` copies a file in; after `--`,
`-f` formats the disk, `-q` powers off when done, `run '<cmd> args'` runs it.
Programs from `examples/` (`cd examples && make`) can be copied in the same way.
Note: the skeleton passes 0 of 80 tests until syscalls/argument passing exist.

## Project 3 — vm

```sh
cd vm && make -j8 && make check
```
Same pattern as userprog; `make check` passes `--swap-size=4` automatically (add it yourself when running by hand).

## Project 4 — filesys

```sh
cd filesys && make -j8 && make check
```

## Debugging with GDB

```sh
# terminal 1
cd threads/build && pintos --qemu --gdb -- -q run alarm-single
# terminal 2
brew install i386-elf-gdb     # untested here
pintos-gdb kernel.o           # then: target remote localhost:1234
```
Backtraces from a panic: `backtrace kernel.o 0xADDR ...` (needs `utils/backtrace`;
it uses the Perl script, no extra install).

## Troubleshooting

| Symptom | Fix |
|---|---|
| `Compiler (i686-elf-gcc) not found` | `source env.sh`; `brew install i686-elf-gcc` |
| `undefined reference to read/write/strlen` linking user tests | Stale objects from the host `ar`. `make clean` in that project, rebuild (Make.config now uses `i686-elf-ar`) |
| `make check`: `cd: build: No such file` | You ran `make clean`; run plain `make` first |
| Pintos hangs with no output in Terminal | Fixed: `utils/pintos` put QEMU in a background process group, so the tty stopped it. It no longer does when stdin is a terminal |
| No QEMU window appears | Pintos only opened a window when `$DISPLAY` was set (X11), which macOS lacks, so it ran headless with output in the terminal. `utils/pintos` now opens a window on macOS; pass `-v` for headless. The window closes when the kernel powers off (`-q`); drop `-q` to keep it open |
| `Run didn't start up properly: no "Pintos booting"` | Simulator is bochs; check `SIMULATOR = --qemu` in `<project>/Make.vars` |
| `stropts.h not found` building utils | Expected on macOS; `squish-pty`/`squish-unix` are skipped (only matter for Bochs terminal / serial piping) |
| `%c expects int` warning in `devices/ide.c` | Harmless, from provided code with the newer GCC |
