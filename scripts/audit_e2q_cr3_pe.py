#!/usr/bin/env python3
"""Host audit of exact linked E2q terminal CR3 switch opcodes, in a nonwritable executable PE section."""
import sys
from pathlib import Path
from audit_e2k_pe import regions,Reject
# Exact physical switch prefix: CLI; CLD; MOV RSP,RSI; MOV CR3,RDI.
SWITCH=bytes.fromhex("fa fc 48 89 f4 0f 22 df")
def audit(pe):
    hits=[]
    for name,offset,raw in regions(pe):
        start=0
        while True:
            at=raw.find(SWITCH,start)
            if at<0:break
            hits.append((name,offset+at))
            start=at+1
    if len(hits)!=1:
        raise Reject(f"unique executable terminal CR3 opcode sequence count={len(hits)}")
    return hits[0]
def run(path):
    data=Path(path).read_bytes()
    sec,offset=audit(data)
    print(f"E2Q_LINKED_PE_REAL_CR3_SWITCH PASS offset=0x{offset:x} "
          "CLI_CLD_STACK_FIRST_CR3_SECOND=YES executable_nonwritable=YES")
    corrupted=bytearray(data)
    corrupted[offset+7]^=1
    try:audit(corrupted)
    except Reject:print("E2Q_LINKED_PE_CR3_NEGATIVE_CONTROL PASS")
    else:raise Reject("mutated CR3 opcode accepted")
if __name__=="__main__":
    if len(sys.argv)!=2:raise SystemExit("usage: audit_e2q_cr3_pe.py <loader.efi>")
    try:run(sys.argv[1])
    except (OSError,Reject) as e:
        print(f"E2Q_LINKED_CR3_SWITCH_REJECTED: {e}",file=sys.stderr)
        raise SystemExit(1)
