"""E2r-C4 independent 25-target QMP witness for dormant descriptor code RX.

This adds exact linked LGDT/LIDT/LTR code-span page provenance to the 23
E2r-C2 mappings. No descriptor is activated; no kernel code runs.
"""
from e2r_new_tree import (decode_manifest as decode_prior,
                           verify_new as verify_prior,
                           RECORD_LEN,COUNT as PRIOR,PACKET as PRIOR_PACKET)
from e2j_targeted import witness_one
from e2o_new_tree import walk_new

COUNT=25
PACKET=10+COUNT*RECORD_LEN

def decode_manifest(data):
    if len(data)!=PACKET or data[9]!=COUNT:
        raise ValueError("E2r-C4 exact 25-target manifest required")
    prefix=bytearray(data[:PRIOR_PACKET])
    prefix[9]=PRIOR
    root,used,targets=decode_prior(prefix)
    for i in range(PRIOR,COUNT):
        offset=10+i*RECORD_LEN
        targets.append({
            "va":int.from_bytes(data[offset:offset+8],"little"),
            "pa":int.from_bytes(data[offset+8:offset+16],"little"),
            "rights":data[offset+16],
        })
    return root,used,targets

def descriptor_code_span(targets):
    if len(targets)!=COUNT:
        raise ValueError("missing exact descriptor code targets")
    start,last=targets[23],targets[24]
    begin,end=start["va"],last["va"]
    if (start["rights"]!=1 or last["rights"]!=1 or
        start["pa"]!=begin or last["pa"]!=end or
        not 0x100000<=begin<1<<32 or end<begin or end>=1<<32 or
        end-begin>=256 or (end>>12)-(begin>>12)>1):
        raise ValueError("untrusted descriptor code span or PTE rights")
    return begin,end+1

def verify_new(root,used,targets,records,old_root,reader,old_reader):
    begin,end=descriptor_code_span(targets)
    proof=verify_prior(root,used,targets[:PRIOR],records,old_root,reader,old_reader)
    for item in targets[23:]:
        old=witness_one(old_root,item["va"],old_reader)
        if old["effective_user"] or not old["effective_execute"]:
            raise ValueError("descriptor-code not supervisor RX under old CR3")
        if walk_new(root,used,item["va"],reader)!=(item["pa"],1):
            raise ValueError("descriptor-code not equal-physical RX under new PML4")
    proof.update({
        "new_target_count":COUNT,
        "descriptor_stub_begin":f"0x{begin:x}",
        "descriptor_stub_end":f"0x{end:x}",
        "descriptor_code_old_new_rx":True,
        "descriptor_activated":False,
        "kernel_entry_executed":False,
    })
    return proof
