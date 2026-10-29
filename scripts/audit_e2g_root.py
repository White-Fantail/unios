#!/usr/bin/env python3
"""Independent decoding of host-produced E2g descriptor fixture.
Not an independent VM root PML4 or virtual-to-physical translation audit."""
import struct
import sys
from pathlib import Path
CR3=0x120000
def gate(blob):
    if len(blob)!=144: raise ValueError("wrong map length")
    roots=[]
    reserved=(0x200000,0x208000)
    for offset in range(0,len(blob),48):
        kind=struct.unpack_from("<I",blob,offset)[0]
        start=struct.unpack_from("<Q",blob,offset+8)[0]
        pages=struct.unpack_from("<Q",blob,offset+24)[0]
        if not pages or start%4096: raise ValueError("bad descriptor")
        end=start+pages*4096
        if start<=CR3 and CR3+4096<=end:
            roots.append(kind)
        if kind==4 and start<reserved[1] and reserved[0]<end:
            raise ValueError("root overlaps reservation")
    if roots!=[4]:raise ValueError("wrong owner kind or duplicate")
    return True
if __name__=="__main__":
    if len(sys.argv)!=2: raise SystemExit("usage audit_e2g_root.py <bin>")
    data=bytearray(Path(sys.argv[1]).read_bytes())
    assert gate(data)
    print("E2G_INDEPENDENT_HOST_ROOT_AUDIT PASS descriptor_kind=BootServicesData")
    data[0:4]=(7).to_bytes(4,"little")
    try: gate(data)
    except ValueError:
        print("E2G_ROOT_NEGATIVE_CONTROL PASS conventional_RAM_rejected")
    else: raise SystemExit("ERROR: conventional RAM root accepted")
