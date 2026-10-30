"""E2r-C6 strict independent paused-QMP CPU/physical BootContext witness.

All inputs are *observations* (QMP register strings and physical RAM),
never a guest-asserted success marker. No final exit-65 claim here.
"""
import re
from e2o_new_tree import MAIN_STACK, CONTEXT, DESCRIPTORS
from e2r_live_cpu import field,table

MAGIC=0x48314c534f494e55
MAIN_TOP=MAIN_STACK+4*4096
BASE=0xffffffff80000000
LIMIT=0xffffffff84000000

def reg(text,name):
    pattern=r"\b"+name+r"=([0-9a-fA-F]{8,16})\b"
    values=re.findall(pattern,text,re.I)
    if len(values)!=1:raise ValueError("missing/ambiguous QMP "+name)
    return int(values[0],16)

def verify_kernel(regs,root,entry,rx_pa,read_private):
    actual_cr3=reg(regs,"CR3")
    rip=reg(regs,"RIP")
    rsp=reg(regs,"RSP")
    if actual_cr3!=root or root<0x100000 or root&(4096-1):
        raise ValueError("high-half kernel ran without private CR3")
    if not BASE<=entry<LIMIT or not entry<=rip<entry+4096:
        raise ValueError("CPU never executed audited high-half entry RX page")
    if not MAIN_STACK<rsp<MAIN_TOP or rsp&7:
        raise ValueError("CPU not using owned high-half RW stack")
    if table(regs,"GDT")!=(DESCRIPTORS,39):
        raise ValueError("CPU GDTR changed before kernel entry")
    if table(regs,"IDT")!=(DESCRIPTORS+0x200,511):
        raise ValueError("CPU IDTR changed before kernel entry")
    for name,expected in (("CS",8),("SS",16),("DS",16),("ES",16),("TR",24)):
        if field(regs,name,4)!=expected:
            raise ValueError("kernel CPU selector "+name+" invalid")
    tr=re.findall(r"(?m)^TR\s*=\s*([0-9a-fA-F]{4})\s+([0-9a-fA-F]{16})\b",regs)
    if len(tr)!=1 or int(tr[0][1],16)!=DESCRIPTORS+0x100:
        raise ValueError("kernel CPU high-half TSS cache invalid")
    mapped=read_private(rip)
    if mapped!=(rx_pa+(rip-entry),1):
        raise ValueError("kernel RIP physical executable leaf differs from staged ELF")
    stacked=read_private(rsp)
    if not stacked or stacked[1]!=2:
        raise ValueError("actual kernel RSP not mapped supervisor RW/NX")
    return {"result":"PASS","kernel_cpu_cr3":hex(actual_cr3),
            "kernel_cpu_rip":hex(rip),"kernel_cpu_rsp":hex(rsp),
            "kernel_cpu_active_descriptors":True,
            "kernel_cpu_rx_physical_entry_provenance":True,
            "kernel_cpu_stack_rw_nx":True}

def verify_context(words,root,entry):
    if len(words)!=8 or words[:6]!=[MAGIC,1,entry,MAIN_TOP,root,CONTEXT] or words[6:]!=[0,0]:
        raise ValueError("physical BootContext magic/version/entry/stack/CR3/VA mismatch")
    return {"boot_context_pa_bytes_independently_verified":True,
            "boot_context_expected_va":hex(CONTEXT),
            "boot_context_words_checked":8}
