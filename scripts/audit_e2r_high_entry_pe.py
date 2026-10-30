#!/usr/bin/env python3
"""E2r-B independently reject wrong linked high-half SysV64 tail-jump bytes.

This audits a PE section; it does NOT show execution, installed descriptors,
a CPU CR3 transition, kernel RIP, or a completed L1 boot.
"""
import sys
from pathlib import Path
from audit_e2k_pe import regions, Reject

# CLI; CLD; MOV RSP,RDI; AND RSP,-16; SUB RSP,8;
# MOV QWORD PTR [RSP],0; MOV RDI,RSI; JMP RDX.
# SysV64 initial RSP%16=8, RDI contains boot-context pointer.
SEQUENCE = bytes.fromhex(
    "fa fc 48 89 fc 48 83 e4 f0 48 83 ec 08 "
    "48 c7 04 24 00 00 00 00 48 89 f7 ff e2"
)
PREFIX = bytes.fromhex("fa fc 48 89 fc")

def matches(pe):
    hits = []
    prefix_count = 0
    for name, offset, raw in regions(pe):
        prefix_count += raw.count(PREFIX)
        start = 0
        while True:
            pos = raw.find(SEQUENCE, start)
            if pos < 0:
                break
            hits.append((name, offset + pos))
            start = pos + 1
    return hits, prefix_count

def audit(pe):
    hits, count = matches(pe)
    if len(hits) != 1 or count != 1:
        raise Reject(f"exact unique SysV64 high-half final-jump count={len(hits)} prefixes={count}")
    return hits[0][1]

def reject_mutation(data, pos, label):
    altered = bytearray(data)
    altered[pos] ^= 1
    try:
        audit(bytes(altered))
    except Reject:
        print(f"E2R_B_NEGATIVE PASS {label}")
        return
    raise Reject(f"mutated {label} accepted")

def run(path, absent=False):
    data = Path(path).read_bytes()
    if absent:
        hits, count = matches(data)
        if hits or count:
            raise Reject("default nonfatal preflight unexpectedly contains high-half entry stub")
        print("E2R_B_SAFE_PREFLIGHT_ABSENT PASS")
        return
    at = audit(data)
    print(f"E2R_B_LINKED_PE_HIGH_ENTRY PASS offset=0x{at:x} SYSV64_JUMP_RSP_ALIGNMENT=8 UNCALLED_ONLY=YES")
    reject_mutation(data, at+4, "stack_register")
    reject_mutation(data, at+11, "stack_alignment")
    reject_mutation(data, at+24, "tail_jump")
    print("E2R_B_LINKED_PE_NEGATIVES PASS 3/3; HIGH_HALF_QEMU_EXECUTION=NOT_RUN")

if __name__ == "__main__":
    if len(sys.argv) == 2:
        args = (sys.argv[1], False)
    elif len(sys.argv) == 3 and sys.argv[1] == "--absent":
        args = (sys.argv[2], True)
    else:
        raise SystemExit("usage: audit_e2r_high_entry_pe.py [--absent] <loader.efi>")
    try:
        run(*args)
    except (OSError, Reject) as exc:
        print(f"E2R_B_LINKED_PE_REJECTED: {exc}", file=sys.stderr)
        raise SystemExit(1)
