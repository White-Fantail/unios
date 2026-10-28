#!/usr/bin/env python3
"""Independent decoder of synthetic E2f GDT/TSS/IDT fixture.
Does not establish installed GDTR/IDTR/TR in a QEMU guest."""
import struct
import sys
from pathlib import Path
BASE=0xffffffff83fc0000
MAIN=0xffffffff83fe5000
DF=0xffffffff83ff5000
NORMAL=0x14000200
DF_HANDLER=0x14000400
def u64(d,o): return struct.unpack_from("<Q",d,o)[0]
def u32(d,o): return struct.unpack_from("<I",d,o)[0]
def u16(d,o): return struct.unpack_from("<H",d,o)[0]
def audit(d):
    if len(d)!=4096: raise ValueError("descriptor reservation not 4KiB")
    if u64(d,0)!=0 or u64(d,8)!=0x00af9a000000ffff or u64(d,16)!=0x00cf92000000ffff:
        raise ValueError("invalid ring0 GDT descriptors")
    low=u64(d,24); high=u64(d,32)
    limit=low&0xffff|((low>>48)&15)<<16
    base=((low>>16)&0xffffff)|((low>>56)&255)<<24|(high&0xffffffff)<<32
    if ((low>>40)&255)!=0x89 or limit!=103 or base!=BASE+0x100 or high>>32:
        raise ValueError("invalid 64-bit TSS descriptor")
    if u64(d,0x100+4)!=MAIN or u64(d,0x100+36)!=DF or u16(d,0x100+102)!=104:
        raise ValueError("RSP0 / DF IST1 / IOPB invalid")
    if any(d[40:0x100]) or any(d[0x100+104:0x200]) or any(d[0x400:]):
        raise ValueError("reserved bytes not zeroed")
    for vector in range(32):
        o=0x200+vector*16
        target=u16(d,o)|(u16(d,o+6)<<16)|(u32(d,o+8)<<32)
        want=DF_HANDLER if vector==8 else NORMAL
        if (target!=want or u16(d,o+2)!=8 or d[o+4]!=(1 if vector==8 else 0)
                or d[o+5]!=0x8e or u32(d,o+12)!=0):
            raise ValueError(f"bad IDT vector {vector}")
    return True
if __name__=="__main__":
    if len(sys.argv)!=2: raise SystemExit("usage: audit_e2f_descriptors.py <fixture>")
    raw=bytearray(Path(sys.argv[1]).read_bytes())
    audit(raw)
    print("E2F_INDEPENDENT_DESCRIPTOR_AUDIT PASS GDT=5slots TSS=104B IDT=32v DF_IST=1")
    raw[0x200+8*16+4]=0
    try: audit(raw)
    except ValueError:
        print("E2F_DESCRIPTOR_NEGATIVE_CONTROL PASS corrupted_DF_IST_rejected")
    else: raise SystemExit("ERROR: independently corrupted DF IST was accepted")
