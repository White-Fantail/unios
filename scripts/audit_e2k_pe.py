#!/usr/bin/env python3
"""Independent host byte audit of E2k *linked PE* executable code.

It is not evidence of active IDT installation or actual CPU faults.
Read only standard DOS/PE/COFF headers and executable sections.
"""
import struct
import sys
from pathlib import Path

BYTES=11
EXPECTED={
    "generic":bytes.fromhex("fa 66 ba f4 00 b0 31 ee f4 eb fd"),
    "double_fault":bytes.fromhex("fa 66 ba f4 00 b0 32 ee f4 eb fd"),
}
class Reject(ValueError):pass

def regions(pe):
    if len(pe)<0x40 or pe[:2]!=b"MZ":
        raise Reject("missing DOS header")
    off=struct.unpack_from("<I",pe,0x3c)[0]
    if off<0x40 or off+24>len(pe) or pe[off:off+4]!=b"PE\0\0":
        raise Reject("missing PE header")
    machine,nsec=struct.unpack_from("<HH",pe,off+4)
    if machine!=0x8664 or not 1<=nsec<=32:
        raise Reject("not bounded x64 PE")
    opt=struct.unpack_from("<H",pe,off+20)[0]
    optstart=off+24
    table=optstart+opt
    if opt<112 or table+nsec*40>len(pe):
        raise Reject("PE optional header/section table invalid")
    if struct.unpack_from("<H",pe,optstart)[0]!=0x20b:
        raise Reject("not PE32+")
    ranges=[]
    for i in range(nsec):
        base=table+i*40
        name=pe[base:base+8].split(b"\0",1)[0]
        raw_size,raw_off,flags=(
            struct.unpack_from("<I",pe,base+16)[0],
            struct.unpack_from("<I",pe,base+20)[0],
            struct.unpack_from("<I",pe,base+36)[0])
        if raw_off+raw_size>len(pe):
            raise Reject("section outside image")
        if flags&0x20000000:  # IMAGE_SCN_MEM_EXECUTE
            if flags&0x80000000:raise Reject("writable executable PE section")
            ranges.append((name,raw_off,pe[raw_off:raw_off+raw_size]))
    if not ranges: raise Reject("no executable PE sections")
    return ranges

def audit(pe):
    sections=regions(pe)
    observed={}
    for name,expected in EXPECTED.items():
        matches=[]
        for sec,raw,blob in sections:
            idx=blob.find(expected)
            while idx>=0:
                matches.append({"section":sec.decode("ascii",errors="replace"),
                                "file_offset":raw+idx})
                idx=blob.find(expected,idx+1)
        if len(matches)!=1:
            raise Reject(f"{name} unique executable fail-stop signature count={len(matches)}")
        observed[name]=matches[0]
    if observed["generic"]["file_offset"]==observed["double_fault"]["file_offset"]:
        raise Reject("emergency handlers alias")
    return observed

def run(path):
    data=Path(path).read_bytes()
    observed=audit(data)
    print("E2K_LINKED_PE_EMERGENCY_HANDLERS PASS "
          "generic=0x%x double_fault=0x%x bytes=%d executable_nonwritable=YES"
          %(observed["generic"]["file_offset"],observed["double_fault"]["file_offset"],BYTES))
    # Negative control: corrupt direct-linked PE instruction byte.
    altered=bytearray(data)
    altered[observed["generic"]["file_offset"]+8]^=0x01
    try:audit(altered)
    except Reject:print("E2K_PE_NEGATIVE_CONTROL PASS mutated_hlt_rejected")
    else:raise Reject("corrupt executable fail-stop byte not rejected")

if __name__=="__main__":
    if len(sys.argv)!=2:
        raise SystemExit("usage: audit_e2k_pe.py <loader.efi>")
    try:run(sys.argv[1])
    except (Reject,OSError,struct.error) as exc:
        print("E2K_LINKED_PE_REJECTED:",exc,file=sys.stderr)
        raise SystemExit(1)
