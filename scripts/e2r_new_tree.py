"""E2r-C1: QMP-host independently checks high-entry PE RX span in both roots.

The linked high-entry symbol is NOT run; this is a pre-cut source-to-PTE
attestation only and cannot imply GDTR/IDTR/TR activation or kernel boot.
"""
from e2q_new_tree import (decode_manifest as decode_prior,
                           verify_new as verify_prior,
                           RECORD_LEN, COUNT as PRIOR, PACKET as PRIOR_PACKET)
from e2j_targeted import witness_one
from e2o_new_tree import walk_new

COUNT=23
PACKET=10+COUNT*RECORD_LEN

def decode_manifest(data):
    if len(data)!=PACKET or data[9]!=COUNT:
        raise ValueError("E2r-C1 exact 23-target manifest required")
    prior=bytearray(data[:PRIOR_PACKET])
    prior[9]=PRIOR
    root,used,targets=decode_prior(prior)
    for i in range(PRIOR,COUNT):
        at=10+i*RECORD_LEN
        va=int.from_bytes(data[at:at+8],"little")
        pa=int.from_bytes(data[at+8:at+16],"little")
        rights=data[at+16]
        targets.append({"va":va,"pa":pa,"rights":rights})
    return root,used,targets

def require_high_span(targets):
    if len(targets)!=COUNT:
        raise ValueError("missing high-entry PE witness")
    start,end=targets[21:]
    begin,last=start["va"],end["va"]
    if (start["rights"]!=1 or end["rights"]!=1 or
        start["pa"]!=begin or end["pa"]!=last or
        not 0x100000<=begin<1<<32 or
        last<begin or last>=1<<32 or last-begin>=64 or
        (last>>12)-(begin>>12)>1):
        raise ValueError("E2r-C1 high-entry PE code span not exact bounded low RX")
    return begin,last+1

def verify_new(root,used,targets,records,old_root,reader,old_reader):
    begin,end=require_high_span(targets)
    proof=verify_prior(root,used,targets[:PRIOR],records,old_root,reader,old_reader)
    for item in targets[21:]:
        prior=witness_one(old_root,item["va"],old_reader)
        if prior["effective_user"] or not prior["effective_execute"]:
            raise ValueError("high-entry PE not supervisor RX under old root")
        if walk_new(root,used,item["va"],reader)!=(item["pa"],1):
            raise ValueError("high-entry PE not same-PA RX under new root")
    proof.update({
        "new_target_count":COUNT,
        "high_entry_stub_begin":f"0x{begin:x}",
        "high_entry_stub_end":f"0x{end:x}",
        "high_entry_stub_old_new_rx":True,
        "high_entry_stub_executed":False,
        "kernel_entry_executed":False,
    })
    return proof
