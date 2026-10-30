#!/usr/bin/env python3
"""Independent QEMU 8.2 native -d int proof of kernel-origin #UD.

A guest COM1 string, debug-exit 99, or software INT 6 is insufficient.
Require an actual QEMU CPU exception vector 6, i=0, CPL0, selector 0x08,
and an IP inside the verified high-half ELF entry RX page. Reject resets.
"""
import re
import sys
from pathlib import Path
BASE=0xffffffff80000000
LIMIT=BASE+4096
MAX_BYTES=32*1024*1024

def verify_native_high_ud(raw, entry=BASE):
    if not isinstance(raw,(bytes,bytearray)) or not raw or len(raw)>MAX_BYTES:
        raise ValueError("empty or oversized QEMU native exception log")
    if entry!=BASE:
        raise ValueError("kernel entry not pinned to approved ELF address")
    text=raw.decode("utf-8",errors="replace")
    if re.search(r"triple\s+fault|CPU\s+reset",text,re.I):
        raise ValueError("QEMU reset/triple fault observed")
    matches=list(re.finditer(
        r"\bv=0?6\s+e=([0-9a-fA-F]{4,16})\s+i=0\s+cpl=0\s+"
        r"IP=([0-9a-fA-F]{4}):([0-9a-fA-F]{16})\b",text,re.I))
    valid=[m for m in matches
           if int(m.group(1),16)==0 and int(m.group(2),16)==8 and
              entry<=int(m.group(3),16)<LIMIT]
    if len(valid)!=1:
        raise ValueError("missing/duplicate CPU #UD at actual ring0 high-half ELF entry")
    return {
        "native_qemu_exception":"CPU_vector_6_UD2",
        "native_vector":6,"native_cpu_origin_i":0,
        "native_ring":0,"native_selector":8,
        "native_fault_rip":hex(int(valid[0].group(3),16)),
        "native_high_half_ud_events":1,"no_qemu_reset":True,
    }

def run(path):
    source=Path(path).read_bytes()
    proof=verify_native_high_ud(source)
    print(f"E2R_E1_NATIVE_HIGH_HALF_UD PASS vector=6 selector=8 rip={proof['native_fault_rip']}")
    # Mutations must fail without guest success oracle assistance.
    candidates=[
      (b"v=06",b"v=03","wrong_exception_vector"),
      (b"i=0",b"i=1","software_interrupt"),
      (b"cpl=0",b"cpl=3","wrong_privilege"),
      (b"IP=0008:",b"IP=0038:","firmware_selector"),
    ]
    for old,new,label in candidates:
        if old not in source:raise ValueError(f"cannot build negative control {label}")
        mutant=source.replace(old,new)
        try:verify_native_high_ud(mutant)
        except ValueError:
            print(f"E2R_E1_UD_NEGATIVE PASS {label}")
        else:raise ValueError(f"mutated native exception accepted: {label}")

if __name__=="__main__":
    if len(sys.argv)!=2:raise SystemExit("usage: audit_e2r_high_ud_trace.py <QEMU -d int trace>")
    try:run(sys.argv[1])
    except (OSError,ValueError) as err:
        print(f"E2R_E1_UD_TRACE_REJECTED: {err}",file=sys.stderr)
        raise SystemExit(1)
