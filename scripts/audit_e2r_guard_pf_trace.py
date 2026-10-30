#!/usr/bin/env python3
"""E2r-E2: require genuine QEMU CPU #PF on the UNMAPPED main-stack guard.

The host also independently QMP-walks that guard in the private PML4.
Guest UART text or QEMU debug-exit byte 0x31 alone never proves a fault.
"""
import re,sys
from pathlib import Path
from e2o_new_tree import MAIN_GUARD
BASE=0xffffffff80000000
MAX_BYTES=32*1024*1024
def verify_native_high_guard_pf(raw):
    if not isinstance(raw,(bytes,bytearray)) or not raw or len(raw)>MAX_BYTES:
        raise ValueError("missing/oversized native QEMU fault log")
    text=raw.decode("utf8",errors="replace")
    if re.search(r"triple\s+fault|CPU\s+reset",text,re.I):
        raise ValueError("reset/triple fault instead of bounded #PF")
    pat=r"\bv=0?e\s+e=([0-9a-fA-F]{4,16})\s+i=0\s+cpl=0\s+IP=([0-9a-fA-F]{4}):([0-9a-fA-F]{16})\b"
    events=list(re.finditer(pat,text,re.I))
    genuine=[]
    for event in events:
        if int(event.group(1),16)!=0 or int(event.group(2),16)!=8:
            continue
        rip=int(event.group(3),16)
        if not BASE<=rip<BASE+4096:
            continue
        # CR2 must come from QEMU's CPU register dump, adjacent to this
        # recorded hardware fault; no unrelated earlier CR2 is accepted.
        tail=text[event.end():event.end()+4000]
        addr=re.search(r"\bCR2=([0-9a-fA-F]{8,16})\b",tail)
        if addr and int(addr.group(1),16)==MAIN_GUARD:
            genuine.append(rip)
    if len(genuine)!=1:
        raise ValueError("no unique high-half ring0 supervisor read #PF on owned guard")
    return {"native_vector":14,"native_fault_error":0,
            "native_cpu_origin_i":0,"native_ring":0,"native_selector":8,
            "native_fault_rip":hex(genuine[0]),"native_cr2":hex(MAIN_GUARD),
            "native_high_half_guard_faults":1,"no_qemu_reset":True}
def run(path):
    raw=Path(path).read_bytes()
    proof=verify_native_high_guard_pf(raw)
    print("E2R_E2_NATIVE_HIGH_HALF_MAIN_GUARD_PF PASS vector=14 "
          f"cr2={proof['native_cr2']} rip={proof['native_fault_rip']}")
    controls=((b"v=0e",b"v=06","wrong_vector"),
              (b"e=0000",b"e=0002","write_fault"),
              (b"i=0",b"i=1","software_interrupt"),
              (b"IP=0008:",b"IP=0038:","firmware_selector"))
    for old,new,label in controls:
        if old not in raw:raise ValueError("missing negative candidate "+label)
        mutant=raw.replace(old,new)
        try:verify_native_high_guard_pf(mutant)
        except ValueError:print("E2R_E2_PF_NEGATIVE PASS "+label)
        else:raise ValueError("mutated #PF trace accepted: "+label)
if __name__=="__main__":
    if len(sys.argv)!=2:raise SystemExit("usage: audit_e2r_guard_pf_trace.py <native-qemu-int-log>")
    try:run(sys.argv[1])
    except (OSError,ValueError) as err:
        print("E2R_E2_PF_TRACE_REJECTED: "+str(err),file=sys.stderr)
        raise SystemExit(1)
