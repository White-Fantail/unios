#!/usr/bin/env python3
"""E2m fail-closed native QEMU -d int evidence of CPU-generated #DF.

A guest's isa-debug-exit byte alone cannot prove the exception's origin.
QEMU's independent nested-fault trace must show a contributory #GP
while delivering a #GP, followed by vector 8 with zero error code.
No QEMU trace can prove portable hardware correctness.
"""
import re
import sys
from pathlib import Path

MAX_TRACE_BYTES=32*1024*1024

def verify_native_df_trace(raw):
    if not isinstance(raw,(bytes,bytearray)) or not raw or len(raw)>MAX_TRACE_BYTES:
        raise ValueError("QEMU native exception trace empty or oversized")
    text=raw.decode("utf-8",errors="replace")
    # Source of actual CPU #DF must be second contributory fault:
    # QEMU -d int "check_exception old: 0xd new 0xd" (QEMU 8.x).
    nested=list(re.finditer(
        r"check_exception\s+old:\s*0x0*d\s+new:?\s*0x0*d\b",
        text,re.I))
    if not nested:
        raise ValueError("no independently logged GP-during-GP exception")
    # CPU #DF pushes architectural zero error code; software INT 8
    # is not accepted by the combination of these two requirements.
    events=list(re.finditer(
        r"\bv=0?8\s+e=0{1,16}\s+i=0\b",text,re.I))
    if not any(e.start()>n.end() for n in nested for e in events):
        raise ValueError("no zero-error-code CPU DF event after nested GP")
    # QEMU could reset after triple fault; never accept that as evidence.
    if re.search(r"triple\s+fault|CPU\s+reset",text,re.I):
        raise ValueError("QEMU reset/triple fault observed")
    return {"nested_gp_count":len(nested),
            "hardware_df_zero_error_events":len(events),
            "qemu_native_exception_trace":"GP_during_GP_then_vector8_error0"}

def run(filename):
    data=Path(filename).read_bytes()
    proof=verify_native_df_trace(data)
    print("E2M_QEMU_NATIVE_REAL_DF_TRACE PASS "
          f"nested_gp={proof['nested_gp_count']} "
          f"df_error_zero={proof['hardware_df_zero_error_events']}")
    # A corrupted native trace must be rejected, independent of exit.
    first=re.search(rb"check_exception\s+old:\s*0x0*d\s+new:?\s*0x0*d\b",data,re.I)
    if not first:raise ValueError("lost native nested fault")
    bad=data[:first.start()]+b"check_exception old: 0xd new 0x6"+data[first.end():]
    try:verify_native_df_trace(bad)
    except ValueError:print("E2M_QEMU_NATIVE_TRACE_NEGATIVE_CONTROL PASS")
    else:raise ValueError("corrupted CPU exception provenance accepted")

if __name__=="__main__":
    if len(sys.argv)!=2:
        raise SystemExit("usage: audit_e2m_df_trace.py <QEMU -d int trace>")
    try:run(sys.argv[1])
    except (OSError,ValueError) as error:
        print(f"E2M_QEMU_NATIVE_DF_REJECTED: {error}",file=sys.stderr)
        raise SystemExit(1)
