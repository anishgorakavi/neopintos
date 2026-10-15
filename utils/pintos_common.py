"""Shared helpers for the Pintos host tools.

This module replaces ``Pintos.pm``.  It knows how to read and write the
virtual disks Pintos boots from: the master boot record (MBR) with its
kernel command line and partition table, the partitions themselves, and
the low-level file helpers the tools share.
"""

from __future__ import annotations

import argparse
import math
import os
import re
import struct
import sys
from dataclasses import dataclass
from typing import (BinaryIO, Callable, Dict, List, NamedTuple, NoReturn,
                    Optional, Sequence, Tuple)

# Number of bytes available for the loader at the beginning of the MBR.
# The kernel command-line arguments follow the loader.
LOADER_SIZE = 314

SECTOR_SIZE = 512
CHUNK_SIZE = 4096
MBR_SIGNATURE = 0xAA55
MAX_COMMAND_LINE = 128

# Partition roles, in the order they are laid out on a disk, and the
# partition type byte that identifies each role in the partition table.
ROLE_ORDER: Tuple[str, ...] = ('kernel', 'filesys', 'scratch', 'swap')
ROLE_TYPES: Dict[str, int] = {
    'kernel': 0x20,
    'filesys': 0x21,
    'scratch': 0x22,
    'swap': 0x23,
}
TYPE_ROLES: Dict[int, str] = {v: k for k, v in ROLE_TYPES.items()}

ALIGNMENTS = ('bochs', 'full', 'none')


class PintosError(Exception):
    """A fatal error to report to the user, without a traceback."""


class Geometry(NamedTuple):
    """IDE disk geometry: heads, and sectors per track."""

    heads: int = 16
    sectors: int = 63

    @property
    def cylinder_sectors(self) -> int:
        """Number of sectors in one cylinder."""
        return self.heads * self.sectors


@dataclass
class Partition:
    """One partition of a Pintos disk.

    A partition that is still to be copied into a new disk has ``file``,
    ``offset`` and ``nbytes`` set: its contents are ``nbytes`` bytes of
    ``file`` starting at byte ``offset`` (``file`` may be ``/dev/zero``).

    A partition that lives on a virtual disk has ``disk``, ``start`` (first
    sector within the disk) and ``sectors`` set.  ``assemble_disk`` fills
    these in for every partition it copies, where ``sectors`` is usually
    larger than ``nbytes / 512`` because of padding.
    """

    file: Optional[str] = None
    offset: int = 0
    nbytes: int = 0
    disk: Optional[str] = None
    start: int = 0
    sectors: int = 0


def warn(message: str) -> None:
    """Print a warning to standard error."""
    print(f'warning: {message}', file=sys.stderr)


# --------------------------------------------------------------------------
# Command-line support shared by the tools.

class UsageParser(argparse.ArgumentParser):
    """An ``ArgumentParser`` that raises ``PintosError`` instead of exiting."""

    def error(self, message: str) -> NoReturn:
        """Report a command-line error."""
        raise PintosError(message)


class OrderedAction(argparse.Action):
    """Record options in command-line order, in ``namespace.ordered``.

    Some options only make sense in the order they were given (``-a``
    applies to the preceding ``-p`` or ``-g``, and a partition may have
    only one source), so those are collected as ``(kind, value)`` pairs.
    For an option declared with ``nargs=0``, ``const`` is the whole pair;
    otherwise ``const`` is the kind and the option's argument the value.
    """

    def __call__(self, parser, namespace, values, option_string=None):
        """Append this option's entry to ``namespace.ordered``."""
        if self.nargs == 0:
            entry = self.const
        else:
            entry = (self.const, values)
        namespace.ordered.append(entry)


def split_kernel_args(argv: Sequence[str]
                      ) -> Tuple[List[str], Optional[List[str]]]:
    """Split ``argv`` at the first ``--``.

    Returns ``(options, kernel_args)``.  If there is no ``--``, then
    ``kernel_args`` is ``None`` and ``options`` is all of ``argv``.
    """
    if '--' not in argv:
        return list(argv), None
    cut = list(argv).index('--')
    return list(argv[:cut]), list(argv[cut + 1:])


def run_tool(main: Callable[[List[str]], int]) -> NoReturn:
    """Run ``main`` on the command line and exit, reporting errors."""
    try:
        status = main(sys.argv[1:])
    except PintosError as error:
        print(error, file=sys.stderr)
        status = 1
    except OSError as error:
        if error.filename is not None:
            print(f'{error.filename}: {error.strerror}', file=sys.stderr)
        else:
            print(error, file=sys.stderr)
        status = 1
    except KeyboardInterrupt:
        status = 130
    sys.stdout.flush()
    sys.exit(status)


# --------------------------------------------------------------------------
# Partitions.

def parse_geometry(text: str) -> Geometry:
    """Parse a ``--geometry`` argument: ``HEADS,SECTORS`` or ``zip``."""
    if text == 'zip':
        return Geometry(64, 32)
    match = re.fullmatch(r'(\d+)[,\s]+(\d+)', text)
    if match is None:
        raise PintosError('bad syntax for geometry')
    heads, sectors = int(match.group(1)), int(match.group(2))
    if heads > 255:
        raise PintosError('heads limited to 255')
    if sectors > 63:
        raise PintosError('sectors per track limited to 63')
    return Geometry(heads, sectors)


def check_alignment(text: str) -> str:
    """Validate an ``--align`` argument and return it."""
    if text not in ALIGNMENTS:
        raise PintosError(f'unknown alignment type "{text}"')
    return text


def register_partition(parts: Dict[str, Partition], role: str, source: str,
                       arg: str) -> None:
    """Set ``role`` of ``parts`` from a ``--ROLE[-SOURCE]=ARG`` option.

    ``source`` is ``file``, ``from`` or ``size``; see ``set_partition``.
    """
    if role in parts:
        raise PintosError(f"can't have two sources for {role} partition")
    set_partition(parts, role, source, arg)


def set_partition(parts: Dict[str, Partition], role: str, source: str,
                  arg: str) -> None:
    """Make partition ``role`` of ``parts`` come from ``source``.

    ``source`` is one of ``file`` (``arg`` is a file to copy), ``from``
    (``arg`` is a disk whose ``role`` partition is copied), or ``size``
    (``arg`` is the size in megabytes of an empty partition).
    """
    part = parts[role] = Partition()
    if source == 'file':
        if read_mbr(arg) is not None:
            warn(f'{arg} looks like a partitioned disk '
                 f'(did you want --{role}-from={arg} or --disk={arg}?)')
        part.file = arg
        part.nbytes = os.path.getsize(arg)
    elif source == 'from':
        found = read_partition_table(arg).get(role)
        if found is None:
            raise PintosError(f'{arg}: does not contain {role} partition')
        part.file = arg
        part.offset = found.start * SECTOR_SIZE
        part.nbytes = found.sectors * SECTOR_SIZE
    elif source == 'size':
        if not re.fullmatch(r'\d+(\.\d+)?|\.\d+', arg):
            raise PintosError(f'{arg}: not a valid size in MB')
        part.file = '/dev/zero'
        part.nbytes = math.ceil(float(arg) * 1024 * 1024)
    else:
        raise ValueError(f'unknown partition source {source!r}')


# --------------------------------------------------------------------------
# Reading and writing disks.

def assemble_disk(disk: BinaryIO, disk_name: str,
                  parts: Dict[str, Partition], *,
                  align: str = 'bochs',
                  geometry: Geometry = Geometry(),
                  disk_format: str = 'partitioned',
                  loader: Optional[bytes] = None,
                  args: Sequence[str] = ()) -> None:
    """Write a virtual disk containing the partitions in ``parts``.

    ``disk`` is the (unbuffered, writable) output file, named ``disk_name``
    for error messages; it is closed when done.  Each partition in
    ``parts`` must have ``file``, ``offset`` and ``nbytes`` set, and has
    ``disk``, ``start`` and ``sectors`` filled in.

    ``align`` is ``bochs`` (pad the disk to a whole cylinder), ``full``
    (also align each partition to a cylinder) or ``none``.
    ``disk_format`` is ``partitioned`` (MBR plus partitions) or ``raw`` (a
    single partition, nothing else).  ``loader`` is the bytes to put at the
    start of the MBR and ``args`` the kernel command line.
    """
    if align not in ALIGNMENTS:
        raise ValueError(f'unknown alignment {align!r}')
    if disk_format not in ('partitioned', 'raw'):
        raise ValueError(f'unknown disk format {disk_format!r}')
    align_partitions = align == 'full'
    pad_disk = align == 'bochs'

    if disk_format == 'raw' and len(parts) != 1:
        raise PintosError('must have exactly one partition for raw output')

    # Calculate where each partition goes.
    total_sectors = 0
    if disk_format == 'partitioned':
        total_sectors += geometry.sectors if align_partitions else 1
    for role in ROLE_ORDER:
        part = parts.get(role)
        if part is None:
            continue
        if part.disk:
            raise ValueError(f'{role} partition is already on a disk')
        start = total_sectors
        end = start + div_round_up(part.nbytes, SECTOR_SIZE)
        if align_partitions:
            end = round_up(end, geometry.cylinder_sectors)
        part.disk = disk_name
        part.start = start
        part.sectors = end - start
        total_sectors = end

    # Write the disk.
    if disk_format == 'partitioned':
        mbr = (loader or b'\xcd\x18').ljust(LOADER_SIZE, b'\0')[:LOADER_SIZE]
        mbr += make_kernel_command_line(args)
        mbr += make_partition_table(geometry, parts)
        mbr += struct.pack('<H', MBR_SIGNATURE)
        if len(mbr) != SECTOR_SIZE:
            raise RuntimeError(f'MBR is {len(mbr)} bytes, not {SECTOR_SIZE}')
        write_fully(disk, disk_name, mbr)
        if align_partitions:
            write_zeros(disk, disk_name,
                        SECTOR_SIZE * (geometry.sectors - 1))
    for role in ROLE_ORDER:
        part = parts.get(role)
        if part is None:
            continue
        assert part.file is not None
        with open(part.file, 'rb', buffering=0) as source:
            if part.offset:
                source.seek(part.offset)
            copy_file(source, part.file, disk, disk_name, part.nbytes)
        write_zeros(disk, disk_name,
                    part.sectors * SECTOR_SIZE - part.nbytes)
    if pad_disk:
        padded = round_up(total_sectors, geometry.cylinder_sectors)
        write_zeros(disk, disk_name, (padded - total_sectors) * SECTOR_SIZE)
    disk.close()


def make_partition_table(geometry: Geometry,
                         parts: Dict[str, Partition]) -> bytes:
    """Return the 64-byte MBR partition table for ``parts``."""
    table = b''
    for role in ROLE_ORDER:
        part = parts.get(role)
        if part is None:
            continue
        end = part.start + part.sectors - 1
        table += struct.pack('<B', 0x80 if role == 'kernel' else 0)
        table += pack_chs(part.start, geometry)
        table += struct.pack('<B', ROLE_TYPES[role])
        table += pack_chs(end, geometry)
        table += struct.pack('<II', part.start, part.sectors)
    return table.ljust(64, b'\0')


def make_kernel_command_line(args: Sequence[str]) -> bytes:
    """Return the bytes stored in the MBR after the loader.

    These are the argument count and the NUL-terminated arguments, padded
    to 128 bytes.
    """
    packed = b''.join(os.fsencode(arg) + b'\0' for arg in args)
    if len(packed) > MAX_COMMAND_LINE:
        raise PintosError(f'command line exceeds {MAX_COMMAND_LINE} bytes')
    return struct.pack(f'<I{MAX_COMMAND_LINE}s', len(args), packed)


def pack_chs(lba: int, geometry: Geometry) -> bytes:
    """Pack logical sector ``lba`` as a 3-byte PC partition-table CHS."""
    cylinder, head, sector = lba_to_chs(lba, geometry)
    return struct.pack('<BBB', head, sector | ((cylinder >> 2) & 0xC0),
                       cylinder & 0xFF)


def lba_to_chs(lba: int, geometry: Geometry) -> Tuple[int, int, int]:
    """Convert logical sector ``lba`` to ``(cylinder, head, sector)``.

    See http://en.wikipedia.org/wiki/CHS_conversion.  Sectors that lie
    beyond cylinder 1023 are reported as the largest CHS address, per
    http://www.cgsecurity.org/wiki/Intel_Partition_Table.
    """
    cylinder, remainder = divmod(lba, geometry.cylinder_sectors)
    head, sector = divmod(remainder, geometry.sectors)
    if cylinder <= 1023:
        return cylinder, head, sector + 1
    return 1023, 254, 63


def read_mbr(path: str) -> Optional[bytes]:
    """Return the 512-byte MBR of ``path``, or ``None`` if it has none."""
    with open(path, 'rb', buffering=0) as handle:
        size = os.fstat(handle.fileno()).st_size
        if size == 0:
            raise PintosError(f'{path}: file has zero size')
        if size >= SECTOR_SIZE:
            mbr = read_fully(handle, path, SECTOR_SIZE)
            if struct.unpack_from('<H', mbr, 510)[0] == MBR_SIGNATURE:
                return mbr
    return None


def interpret_partition_table(mbr: bytes, disk: str) -> Dict[str, Partition]:
    """Parse the partition table in ``mbr``, a 512-byte MBR from ``disk``.

    Returns the Pintos partitions found, by role.  ``disk`` is used in
    warnings about entries that are skipped.
    """
    found: Dict[str, Partition] = {}
    for index in range(4):
        entry = mbr[446 + 16 * index:446 + 16 * (index + 1)]
        bootable = entry[0]
        in_use = struct.unpack_from('<I', entry, 0)[0]
        part_type = entry[4]
        start, sectors = struct.unpack_from('<II', entry, 8)
        if not in_use:
            continue
        if bootable not in (0, 0x80):
            warn(f'invalid partition entry {index} in {disk}')
            continue
        role = TYPE_ROLES.get(part_type)
        if role is None:
            warn(f'non-Pintos partition type 0x{part_type:02x} in {disk}')
            continue
        if role in found:
            warn(f'duplicate {role} partition in {disk}')
            continue
        found[role] = Partition(disk=disk, start=start, sectors=sectors)
    return found


def read_partition_table(path: str) -> Dict[str, Partition]:
    """Read the partitions of the disk ``path``; fail if it has none."""
    mbr = read_mbr(path)
    if mbr is None:
        raise PintosError(f'{path}: not a partitioned disk')
    return interpret_partition_table(mbr, path)


def find_file(base_name: str) -> Optional[str]:
    """Look for ``base_name`` in a couple of likely places."""
    for name in (base_name, os.path.join('build', base_name)):
        if os.path.exists(name):
            return name
    return None


def read_loader(name: Optional[str] = None) -> bytes:
    """Return the bootstrap loader, from ``name`` or ``loader.bin``."""
    if name is None:
        name = find_file('loader.bin')
    if name is None:
        raise PintosError('Cannot find loader')
    with open(name, 'rb', buffering=0) as handle:
        size = os.fstat(handle.fileno()).st_size
        if size not in (LOADER_SIZE, SECTOR_SIZE):
            raise PintosError(f'{name}: must be exactly {LOADER_SIZE} or '
                              f'{SECTOR_SIZE} bytes long')
        return read_fully(handle, name, LOADER_SIZE)


# --------------------------------------------------------------------------
# Low-level file helpers.

def div_round_up(x: int, y: int) -> int:
    """Return ``x / y`` rounded up to a whole number."""
    return -(-x // y)


def round_up(x: int, y: int) -> int:
    """Return ``x`` rounded up to a multiple of ``y``."""
    return div_round_up(x, y) * y


def read_fully(handle: BinaryIO, name: str, count: int) -> bytes:
    """Read exactly ``count`` bytes from ``handle`` (named ``name``)."""
    data = b''
    while len(data) < count:
        chunk = handle.read(count - len(data))
        if not chunk:
            raise PintosError(f'{name}: unexpected end of file')
        data += chunk
    return data


def write_fully(handle: BinaryIO, name: str, data: bytes) -> None:
    """Write all of ``data`` to ``handle`` (named ``name``)."""
    view = memoryview(data)
    while view:
        written = handle.write(view)
        if not written:
            raise PintosError(f'{name}: short write')
        view = view[written:]


def write_zeros(handle: BinaryIO, name: str, count: int) -> None:
    """Write ``count`` zero bytes to ``handle`` (named ``name``)."""
    zeros = bytes(CHUNK_SIZE)
    while count > 0:
        chunk = min(CHUNK_SIZE, count)
        write_fully(handle, name, zeros[:chunk])
        count -= chunk


def copy_file(source: BinaryIO, source_name: str, dest: BinaryIO,
              dest_name: str, size: int) -> None:
    """Copy ``size`` bytes from ``source`` to ``dest``."""
    while size > 0:
        chunk = min(CHUNK_SIZE, size)
        write_fully(dest, dest_name, read_fully(source, source_name, chunk))
        size -= chunk
