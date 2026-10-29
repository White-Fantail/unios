"""L1 E2q: independent QMP verification of 21-target private PML4, including
the true finite terminal CR3-switch instruction range. New CR3 is built
by EFI but not active at the second O barrier.
"""
from e2o_new_tree import (decode_manifest as old_decode,
    verify_new as old_verify,walk_new,RECORD_LEN,COUNT as PRIOR,
    PACKET as PRIOR_PACKET)
from e2j_targeted import witness_one
COUNT=21
PACKET=10+COUNT*RECORD_LEN
MARKER=b"\x7fUNI2Q\x7f"
def decode_manifest(data):
    if len(data)!=PACKET:
        raise ValueError("exact E2q extended private-root manifest required")
    root,used,prior=old_decode(data[:PRIOR_PACKET])
    extras=[]
    for i in range(PRIOR,COUNT):
        at=10+i*RECORD_LEN
        va=int.from_bytes(data[at:at+8],"little")
        pa=int.from_bytes(data[at+8:at+16],"little")
        right=data[at+16]
        extras.append({"va":va,"pa":pa,"rights":right})
    return root,used,prior+extras

def verify_new(root,used,targets,records,old_root,reader,old_reader):
    if len(targets)!=COUNT:
        raise ValueError("missing exact terminal code witnesses")
    base=old_verify(root,used,targets[:PRIOR],records,
                    old_root,reader,old_reader)
    start,end=targets[19],targets[20]
    if (start["pa"]!=start["va"] or end["pa"]!=end["va"]
        or start["rights"]!=1 or end["rights"]!=1
        or start["va"]<0x100000 or end["va"]>=1<<32
        or end["va"]<=start["va"]
        or start["va"]>>12 != end["va"]>>12):
        raise ValueError("untrusted terminal CR3 switch code span")
    for t in (start,end):
        old=witness_one(old_root,t["va"],old_reader)
        if old["effective_user"] or not old["effective_execute"]:
            raise ValueError("terminal code not supervisor executable under old CR3")
        actual=walk_new(root,used,t["va"],reader)
        if actual!=(t["va"],1):
            raise ValueError("terminal code not same-linear executable under new CR3")
    base.update({"new_target_count":COUNT,
                 "cr3_terminal_code_begin":f"0x{start['va']:x}",
                 "cr3_terminal_code_end":f"0x{end['va']+1:x}",
                 "cr3_terminal_code_old_new_rx":True})
    return base

import re
def parse_switched_regs(text,expected_root,code_begin,code_end,bridge_base,bridge_end):
    def get(name):
        matches=re.findall(r"\b"+name+r"=([0-9a-fA-F]{8,16})\b",text,re.I)
        if len(matches)!=1:raise ValueError("missing or duplicate "+name+" from CPU registers")
        return int(matches[0],16)
    cr3=get("CR3")
    rip=get("RIP")
    rsp=get("RSP")
    if cr3!=expected_root:
        raise ValueError("CPU never loaded private CR3")
    if not code_begin<=rip<code_end:
        raise ValueError("post-switch CPU RIP escaped audited terminal code")
    if not bridge_base<rsp<=bridge_end or rsp%16:
        raise ValueError("post-switch CPU RSP not in 16-byte-aligned owned bridge stack")
    return {"cpu_live_cr3":f"0x{cr3:x}","cpu_live_rip":f"0x{rip:x}",
            "cpu_live_rsp":f"0x{rsp:x}","cpu_private_cr3_activated":True,
            "kernel_entry_executed":False}
