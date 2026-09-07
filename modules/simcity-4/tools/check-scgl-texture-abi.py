#!/usr/bin/env python3
"""Check the built MinGW DLL against SimCity 4 1.1.610 texture call slots."""

import re
import struct
import subprocess
import sys
from pathlib import Path


class CheckError(Exception):
    """A deterministic validation failure."""


def fail(message: str) -> int:
    print(f"FAIL: {message}")
    return 1


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        return fail("usage: check-scgl-texture-abi.py path/to/SCGL.dll")

    dll = Path(argv[1])
    try:
        symbols = read_symbols(dll)
        data = dll.read_bytes()
        table = vtable_file_offset(data, symbols)

        expected = {
            0x6C: "TexEnv(unsigned int, unsigned int, float const*)",
            0x70: "TexEnv(unsigned int, unsigned int, int)",
            0xD8: "TexStageCombine(eGDTextureStageCombineScaleParamType, eGDTextureStageCombineScaleParam)",
            0xDC: "TexStageCombine(eGDTextureStageCombineOperandType, eGDBlend)",
            0xE0: "TexStageCombine(eGDTextureStageCombineSourceParamType, eGDTextureStageCombineSourceParam)",
            0xE4: "TexStageCombine(eGDTextureStageCombineParamType, eGDTextureStageCombineModeParam)",
        }
        for slot, method in expected.items():
            actual = read_u32(data, table + slot, f"vtable slot {slot:#x}")
            symbol = symbols.get(f"nSCGL::cGDriver::{method}")
            if symbol is None:
                raise CheckError(f"required symbol is missing: {method}")
            if actual != symbol:
                raise CheckError(
                    f"slot {slot:#x}: expected {method}, got {actual:#x}"
                )
    except CheckError as error:
        return fail(str(error))
    except OSError as error:
        return fail(str(error))
    except (struct.error, ValueError) as error:
        return fail(f"invalid PE data: {error}")

    print("PASS: all six SimCity 4 1.1.610 texture ABI slots match")
    return 0


def read_symbols(dll: Path) -> dict[str, int]:
    try:
        result = subprocess.run(
            ["i686-w64-mingw32-nm", "-C", str(dll)],
            check=False,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError as error:
        raise CheckError("i686-w64-mingw32-nm was not found on PATH") from error
    if result.returncode != 0:
        detail = result.stderr.strip() or f"exit status {result.returncode}"
        raise CheckError(f"nm failed: {detail}")

    symbols = {}
    for line in result.stdout.splitlines():
        fields = line.split(maxsplit=2)
        if (
            len(fields) == 3
            and re.fullmatch(r"[0-9a-fA-F]+", fields[0])
            and fields[1] != "A"
        ):
            symbols[fields[2]] = int(fields[0], 16)
    return symbols


def vtable_file_offset(data: bytes, symbols: dict[str, int]) -> int:
    pe = read_u32(data, 0x3C, "PE header offset")
    if pe + 4 > len(data):
        raise CheckError("PE header is outside the input")
    if data[pe : pe + 4] != b"PE\0\0":
        raise CheckError("PE signature is missing")
    if read_u16(data, pe + 4, "machine") != 0x14C:
        raise CheckError("input is not a PE32 i386 image")

    optional = pe + 24
    image_base = read_u32(data, optional + 28, "image base")
    section_count = read_u16(data, pe + 6, "section count")
    sections = optional + read_u16(data, pe + 20, "optional header size")
    try:
        vtable = symbols["vtable for nSCGL::cGDriver"]
    except KeyError as error:
        raise CheckError("required vtable symbol is missing") from error
    if vtable < image_base:
        raise CheckError("vtable address precedes the PE image base")
    table = vtable + 8 - image_base

    for index in range(section_count):
        section = sections + 40 * index
        virtual_size = read_u32(data, section + 8, "section virtual size")
        virtual_address = read_u32(data, section + 12, "section RVA")
        raw_size = read_u32(data, section + 16, "section raw size")
        raw_offset = read_u32(data, section + 20, "section raw offset")
        if virtual_address <= table < virtual_address + max(virtual_size, raw_size):
            return raw_offset + table - virtual_address
    raise CheckError("vtable is not in a file-backed section")


def read_u16(data: bytes, offset: int, label: str) -> int:
    try:
        return struct.unpack_from("<H", data, offset)[0]
    except struct.error as error:
        raise CheckError(f"{label} is outside the input") from error


def read_u32(data: bytes, offset: int, label: str) -> int:
    try:
        return struct.unpack_from("<I", data, offset)[0]
    except struct.error as error:
        raise CheckError(f"{label} is outside the input") from error


if __name__ == "__main__":
    sys.exit(main(sys.argv))
