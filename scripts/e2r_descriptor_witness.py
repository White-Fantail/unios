"""E2r-C2: independently validate the actual EFI-built GDT/IDT/TSS page.

The exact 4096 bytes come from PAUSED QMP physical RAM, cross-compared
against the old-CR3 virtual alias. This is a DORMANT descriptor audit,
NOT CPU GDTR/IDTR/TR state and NOT post-ExitBS proof.
"""
import hashlib
import struct
from e2o_new_tree import DESCRIPTORS, MAIN_STACK, IST_STACK
from e2i_table_walk import PAGE

GDT_CODE=0x00af9a000000ffff
GDT_DATA=0x00cf92000000ffff
MAIN_TOP=MAIN_STACK+4*PAGE
IST_TOP=IST_STACK+4*PAGE

def u16(d,o): return struct.unpack_from("<H",d,o)[0]
def u32(d,o): return struct.unpack_from("<I",d,o)[0]
def u64(d,o): return struct.unpack_from("<Q",d,o)[0]

def verify_descriptor_page(raw, normal, df):
    if len(raw)!=PAGE:
        raise ValueError("descriptor backing must be exactly one 4KiB page")
    if not 0x100000<=normal<1<<32 or not 0x100000<=df<1<<32 or normal==df:
        raise ValueError("wrong independently proven EFI RX handlers")
    if u64(raw,0)!=0 or u64(raw,8)!=GDT_CODE or u64(raw,16)!=GDT_DATA:
        raise ValueError("ring0 GDT code/data mismatch")
    lo,hi=u64(raw,24),u64(raw,32)
    limit=(lo&0xffff)|(((lo>>48)&15)<<16)
    base=((lo>>16)&0xffffff)|(((lo>>56)&0xff)<<24)|((hi&0xffffffff)<<32)
    if ((lo>>40)&0xff)!=0x89 or limit!=103 or base!=DESCRIPTORS+0x100 or hi>>32:
        raise ValueError("64-bit available TSS descriptor invalid")
    if u64(raw,0x100+4)!=MAIN_TOP or u64(raw,0x100+36)!=IST_TOP or u16(raw,0x100+102)!=104:
        raise ValueError("TSS RSP0/IST1/IOPB mismatch")
    if any(raw[40:0x100]) or any(raw[0x100+104:0x200]) or any(raw[0x400:]):
        raise ValueError("reserved descriptor bytes must be zero")
    for vector in range(32):
        p=0x200+16*vector
        dest=u16(raw,p)|(u16(raw,p+6)<<16)|(u32(raw,p+8)<<32)
        target=df if vector==8 else normal
        if (dest!=target or u16(raw,p+2)!=8
            or raw[p+4]!=(1 if vector==8 else 0)
            or raw[p+5]!=0x8e or u32(raw,p+12)!=0):
            raise ValueError(f"IDT vector {vector} invalid")
    return {
        "descriptor_pa_contents_independently_verified":True,
        "descriptor_page_sha256":hashlib.sha256(raw).hexdigest(),
        "descriptor_gdt_slots":5,
        "descriptor_tss_ist1_high_half":True,
        "descriptor_idt_vectors":32,
        "descriptor_activated":False,
        "kernel_entry_executed":False,
    }
