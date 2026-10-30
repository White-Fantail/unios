#!/usr/bin/env python3
"""Offline linked PE audit of *unreachable* x86_64 GDT/IDT/TSS transition.

No CPU activation, QEMU runtime proof or clean kernel boot is implied.
The exact byte contract is pinned to the actual LLVM-linked PE32+ image;\nLLVM emits 66 operand-size prefixes on 16-bit segment-register moves.
"""
from pathlib import Path
import sys
from audit_e2k_pe import regions,Reject

CODE=bytes.fromhex(
    "fa fc 0f 01 17 0f 01 1e 66 b8 18 00 0f 00 d8 6a 08 "
    "48 8d 05 03 00 00 00 50 48 cb 66 b8 10 00 66 8e d8 "
    "66 8e c0 66 8e d0 31 c0 66 8e e0 66 8e e8 49 89 d2 48 89 cf "
    "4c 89 c6 4c 89 ca 41 ff e2"
)
PREFIX=CODE[:8]

def occurrences(data):
    found=[]
    prefixes=0
    for section,offset,raw in regions(data):
        prefixes+=raw.count(PREFIX)
        start=0
        while True:
            index=raw.find(CODE,start)
            if index<0:break
            found.append((section,offset+index))
            start=index+1
    return found,prefixes

def audit(data):
    hits,prefixes=occurrences(data)
    if len(hits)!=1 or prefixes!=1:
        samples=[raw[i:i+72].hex(" ") for _,_,raw in regions(data)
                 for i in range(len(raw)) if raw.startswith(PREFIX,i)][:2]
        raise Reject(f"exact descriptor transition count={len(hits)} prefixes={prefixes}; linked_pe_prefix_samples={samples}")
    return hits[0][1]

def negative(data,at,index,label):
    bad=bytearray(data)
    bad[at+index]^=1
    try:audit(bad)
    except Reject:
        print(f"E2R_C3_NEGATIVE PASS {label}")
        return
    raise Reject(f"corrupt {label} opcode passed linked audit")

def run(path,absent):
    data=Path(path).read_bytes()
    if absent:
        hits,prefixes=occurrences(data)
        if hits or prefixes:
            raise Reject("default E2q EFI contaminated by descriptor activation")
        print("E2R_C3_SAFE_EFI_NO_ACTIVATION_CODE PASS")
        return
    at=audit(data)
    print(f"E2R_C3_LINKED_DESCRIPTOR_CODE PASS offset=0x{at:x} bytes={len(CODE)} GDTR_IDTR_TR_CS_RELOAD=OFFLINE_ONLY")
    for index,label in ((4,"lgdt_operand"),(7,"lidt_operand"),
                        (14,"ltr_selector"),(26,"far_CS_reload"),
                        (len(CODE)-1,"SysV_tail_jump")):
        negative(data,at,index,label)
    print("E2R_C3_LINKED_NEGATIVES PASS 5/5; CPU_DESCRIPTOR_ACTIVATION=NOT_RUN")

if __name__=="__main__":
    if len(sys.argv)==2: path,absent=sys.argv[1],False
    elif len(sys.argv)==3 and sys.argv[1]=="--absent":path,absent=sys.argv[2],True
    else:raise SystemExit("usage: audit_e2r_descriptor_pe.py [--absent] <loader.efi>")
    try:run(path,absent)
    except (OSError,Reject) as err:
        print(f"E2R_C3_LINKED_REJECTED: {err}",file=sys.stderr)
        raise SystemExit(1)
