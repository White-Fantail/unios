#!/usr/bin/env python3
"""Independent walker for HOST-GENERATED L1 PTE fixture, not QEMU guest dump."""
import struct
import sys
from pathlib import Path

BASE = 0xFFFFFFFF80000000
LIMIT = 0xFFFFFFFF84000000
ROOT_PA = 0x01000000
PAGE = 4096
TABLE_LEN = 128 * PAGE
MASK = 0x000FFFFFFFFFF000
NX = 1 << 63
MAIN_GUARD = LIMIT - 0x20000
IST_GUARD = LIMIT - 0x10000
CONTEXT = LIMIT - 0x30000
DESCRIPTORS = LIMIT - 0x40000

class Violation(ValueError):
    pass

def read64(blob, at):
    if at < 0 or at + 8 > len(blob):
        raise Violation("out of bounds")
    return struct.unpack_from("<Q", blob, at)[0]

def from_elf(raw):
    if len(raw) < 64 or raw[:4] != bytes([127,69,76,70]):
        raise Violation("not ELF")
    phoff = read64(raw, 32)
    phentsize, phnum = struct.unpack_from("<HH", raw, 54)
    if phentsize != 56 or not 1 <= phnum <= 32:
        raise Violation("program headers")
    expected = {}
    load_index = 0
    for i in range(phnum):
        off = phoff + i * phentsize
        if off + 56 > len(raw):
            raise Violation("truncated program header")
        kind,flags,_,va,_,_,memsz,_ = struct.unpack_from("<IIQQQQQQ",raw,off)
        if kind != 1:
            continue
        first=va & ~(PAGE-1)
        end=(va+memsz+PAGE-1)&~(PAGE-1)
        rights=(False,False) if flags & 1 else ((True,True) if flags & 2 else (False,True))
        for addr in range(first,end,PAGE):
            expected[addr]=(0x02000000+load_index*0x01000000+(addr-first),rights)
        load_index += 1
    for i in range(2):
        expected[0x21000000+i*PAGE]=(0x21000000+i*PAGE,(True,True))
    for base,pa,n in [(MAIN_GUARD+PAGE,0x22000000,4),
                      (IST_GUARD+PAGE,0x23000000,4),
                      (DESCRIPTORS,0x24000000,1),(CONTEXT,0x25000000,1)]:
        rights=(False,True) if base==CONTEXT else (True,True)
        for i in range(n):
            expected[base+i*PAGE]=(pa+i*PAGE,rights)
    return expected

def audit(blob, expected):
    if len(blob)!=TABLE_LEN:
        raise Violation("wrong table reservation size")
    leaves={}
    tables=set()
    leaf_pa=set()
    def descend(pa, depth, prefix):
        if pa in tables:
            raise Violation("table alias/cycle")
        tables.add(pa)
        off=pa-ROOT_PA
        if off<0 or off+PAGE>TABLE_LEN or off%PAGE:
            raise Violation("table not in root backing")
        shift=[39,30,21,12][depth]
        for i in range(512):
            e=read64(blob,off+i*8)
            if not e&1: continue
            if e&4 or e&(1<<8):
                raise Violation("user/global mapping")
            va=prefix | (i<<shift)
            if depth<3:
                if not e&2 or e&NX or e&(1<<7):
                    raise Violation("intermediate flags")
                descend(e&MASK,depth+1,va)
            else:
                if e&(1<<7):
                    raise Violation("PAT bit not supported")
                write,noexec=bool(e&2),bool(e&NX)
                if write and not noexec:
                    raise Violation("writable executable")
                physical=e&MASK
                if physical in leaf_pa:
                    raise Violation("physical alias")
                leaf_pa.add(physical)
                if va&(1<<47):
                    va|=0xffff000000000000
                leaves[va]=(physical,(write,noexec))
    descend(ROOT_PA,0,0)
    if leaves!=expected:
        raise Violation("mapped leaves mismatch expected ELF+stacks/context")
    if MAIN_GUARD in leaves or IST_GUARD in leaves:
        raise Violation("mapped guard")
    for va,(pa,_) in leaves.items():
        if ROOT_PA<=pa<ROOT_PA+TABLE_LEN:
            raise Violation("page tables exposed")
        if not (BASE<=va<LIMIT or (0x21000000<=va<0x21002000 and va==pa)):
            raise Violation("unapproved virtual address")
    return len(leaves),len(tables)

def main():
    if len(sys.argv)!=3:
        raise SystemExit("usage: audit_e2c_pte.py <ELF> <PTE-image>")
    wanted=from_elf(Path(sys.argv[1]).read_bytes())
    blob=Path(sys.argv[2]).read_bytes()
    leaves,tables=audit(blob,wanted)
    print(f"E2C_INDEPENDENT_HOST_PTE_AUDIT PASS leaves={leaves} tables={tables} guards=2")
    bad=bytearray(blob)
    slot=((BASE>>39)&511)*8
    struct.pack_into("<Q",bad,slot,read64(bad,slot)|4)
    try:
        audit(bytes(bad),wanted)
    except Violation:
        print("E2C_INDEPENDENT_NEGATIVE_CONTROL PASS user-bit corruption rejected")
    else:
        raise Violation("corrupted root accepted")

if __name__=="__main__":
    main()
