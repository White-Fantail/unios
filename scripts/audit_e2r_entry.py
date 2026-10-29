#!/usr/bin/env python3
"""E2r-A independent *offline* high-half entry/stack/descriptor fixture gate.

This does not prove actual post-CR3 execution or activation of GDTR/IDTR/TR.
"""
import struct
import sys
from pathlib import Path
from audit_e2c_pte import (audit as audit_pte, from_elf, BASE, LIMIT,
                           MAIN_GUARD, IST_GUARD, PAGE, Violation)
from audit_e2f_descriptors import audit as audit_descriptors

def u16(b, off):
    return struct.unpack_from("<H", b, off)[0]

def u32(b, off):
    return struct.unpack_from("<I", b, off)[0]

def u64(b, off):
    return struct.unpack_from("<Q", b, off)[0]

def inspect(elf, pte, descriptors):
    if len(elf) < 64 or elf[:4] != b"\x7fELF" or elf[4:7] != b"\x02\x01\x01":
        raise Violation("ELF64 little-endian required")
    if u16(elf, 16) != 2 or u16(elf, 18) != 62:
        raise Violation("x86_64 ET_EXEC required")
    entry = u64(elf, 24)
    if not BASE <= entry < LIMIT:
        raise Violation("entry outside approved high half")
    phoff, phentsize, phnum = u64(elf, 32), u16(elf, 54), u16(elf, 56)
    if phentsize != 56 or not 1 <= phnum <= 32 or phoff > len(elf) or phnum > (len(elf)-phoff)//56:
        raise Violation("invalid program header bounds")
    executable = []
    segments = []
    bss = 0
    for i in range(phnum):
        off = phoff + i*56
        typ, flags, fileoff, va, pa, filesz, memsz, align = struct.unpack_from("<IIQQQQQQ", elf, off)
        if typ != 1:
            raise Violation("only fixed PT_LOAD headers admitted")
        if flags not in (4, 5, 6):
            raise Violation("segment rights must be exactly R, RX, or RW")
        if not memsz or filesz > memsz or fileoff > len(elf) or filesz > len(elf)-fileoff:
            raise Violation("unbounded segment file/memory extent")
        if va < BASE or va >= LIMIT or memsz > LIMIT-va:
            raise Violation("segment outside high half")
        if va % PAGE or align != PAGE:
            raise Violation("segment page alignment")
        if any(va < end and start < va+memsz for start,end in segments):
            raise Violation("overlapping segment")
        segments.append((va, va+memsz))
        if flags == 5:
            executable.append((va, va+filesz))
        if flags == 6:
            bss += memsz-filesz
    if len(executable) != 1 or not any(start <= entry < end for start,end in executable):
        raise Violation("entry not in file-backed executable PT_LOAD")
    if bss <= 0:
        raise Violation("expected nonempty writable BSS")
    expected = from_elf(elf)
    audit_pte(pte, expected)
    if MAIN_GUARD in expected or IST_GUARD in expected:
        raise Violation("guard mapped")
    try:
        audit_descriptors(descriptors)
    except (ValueError, struct.error) as exc:
        raise Violation(f"descriptor fixture: {exc}") from exc
    return entry, len(expected), bss

def rejected(label, elf, pte, descriptors, mutator):
    a,b,c = bytearray(elf), bytearray(pte), bytearray(descriptors)
    mutator(a,b,c)
    try:
        inspect(a,b,c)
    except (Violation, ValueError, struct.error, IndexError):
        print(f"E2R_A_NEGATIVE PASS {label}")
        return
    raise Violation(f"negative control accepted: {label}")

def main():
    if len(sys.argv) != 4:
        raise SystemExit("usage: audit_e2r_entry.py <kernel-ELF> <E2c-PTE> <E2f-descriptors>")
    elf,pte,desc=(Path(arg).read_bytes() for arg in sys.argv[1:])
    entry,count,bss=inspect(elf,pte,desc)
    print(f"E2R_A_OFFLINE_ENTRY_AUDIT PASS entry=0x{entry:x} leaves={count} bss={bss} descriptors=STAGED_ONLY")
    rejected("noncanonical_entry",elf,pte,desc,lambda a,b,c:struct.pack_into("<Q",a,24,0x0000800000000000))
    rejected("entry_in_unmapped_guard",elf,pte,desc,lambda a,b,c:struct.pack_into("<Q",a,24,MAIN_GUARD))
    rejected("entry_in_nonexec_segment",elf,pte,desc,lambda a,b,c:struct.pack_into("<Q",a,24,u64(a,u64(a,32)+56+16)))
    rejected("user_accessible_root",elf,pte,desc,lambda a,b,c:struct.pack_into("<Q",b,((BASE>>39)&511)*8,u64(b,((BASE>>39)&511)*8)|4))
    rejected("corrupt_df_ist",elf,pte,desc,lambda a,b,c:c.__setitem__(0x200+8*16+4,0))
    print("E2R_A_OFFLINE_NEGATIVES PASS 5/5; QEMU_HIGH_HALF_ENTRY=NOT_RUN")

if __name__ == "__main__":
    try:
        main()
    except (Violation, ValueError, struct.error, OSError) as exc:
        print(f"E2R_A_REJECTED: {exc}",file=sys.stderr)
        raise SystemExit(1)
