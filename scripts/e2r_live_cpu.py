"""E2r-C5: strict *live CPU* QMP attestation after real LGDT/LIDT/LTR.

Requires unique paused-QMP register fields. Fails closed if descriptor
state, CS cache, owned stack, active CR3 or executing PE span mismatch.
Never accepts the kernel boot success code 65 as descriptor test evidence.
"""
import re
from e2o_new_tree import DESCRIPTORS
from e2q_new_tree import parse_switched_regs

def field(text,name,width):
    pattern=r"(?m)^"+name+r"\s*=\s*([0-9a-fA-F]{"+str(width)+r"})\b"
    matches=re.findall(pattern,text)
    if len(matches)!=1:
        raise ValueError("missing or duplicated live CPU field "+name)
    return int(matches[0],16)

def table(text,name):
    p=r"(?m)^"+name+r"=\s*([0-9a-fA-F]{16})\s+([0-9a-fA-F]{8})\b"
    matches=re.findall(p,text)
    if len(matches)!=1:
        raise ValueError("missing/ambiguous live "+name+" base/limit")
    return tuple(int(x,16) for x in matches[0])

def verify_cpu(regs,root,code_begin,code_end,bridge_base,bridge_end):
    live=parse_switched_regs(regs,root,code_begin,code_end,bridge_base,bridge_end)
    if table(regs,"GDT")!=(DESCRIPTORS,39):
        raise ValueError("CPU GDTR not active at high-half owned descriptors")
    if table(regs,"IDT")!=(DESCRIPTORS+0x200,511):
        raise ValueError("CPU IDTR not active at high-half owned descriptors")
    for name,expected in (("CS",8),("SS",16),("DS",16),
                          ("ES",16),("TR",24)):
        if field(regs,name,4)!=expected:
            raise ValueError("CPU "+name+" selector not kernel-owned")
    # TR base must come from our high-half TSS GDT descriptor; selector
    # alone is insufficient, and QEMU prints the live cached TSS base.
    tr_line=re.findall(r"(?m)^TR\s*=\s*([0-9a-fA-F]{4})\s+([0-9a-fA-F]{16})\b",regs)
    if len(tr_line)!=1 or int(tr_line[0][1],16)!=DESCRIPTORS+0x100:
        raise ValueError("CPU TR cached base not owned high-half TSS")
    live.update({
        "cpu_live_gdtr":f"0x{DESCRIPTORS:x}",
        "cpu_live_idtr":f"0x{DESCRIPTORS+0x200:x}",
        "cpu_live_tr":24,"cpu_live_cs":8,"cpu_live_ss":16,
        "cpu_live_tss_base":f"0x{DESCRIPTORS+0x100:x}",
        "cpu_descriptor_activation_verified":True,
        "kernel_entry_executed":False,
    })
    return live
