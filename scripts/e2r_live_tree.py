"""E2r-C5: QMP-host independent 27-target old/new CPU stub closure."""
from e2r_descriptor_new_tree import (decode_manifest as previous_decode,
    verify_new as previous_verify,RECORD_LEN,COUNT as PRIOR,PACKET as PRIOR_PACKET)
from e2j_targeted import witness_one
from e2o_new_tree import walk_new

COUNT=27
PACKET=10+COUNT*RECORD_LEN
MARKER=b"\x7fUNI2R\x7f"

def decode_manifest(data):
    if len(data)!=PACKET or data[9]!=COUNT:
        raise ValueError("E2r-C5 must provide exactly 27 target records")
    base=bytearray(data[:PRIOR_PACKET])
    base[9]=PRIOR
    root,used,targets=previous_decode(base)
    for i in range(PRIOR,COUNT):
        o=10+i*RECORD_LEN
        targets.append({"va":int.from_bytes(data[o:o+8],"little"),
                        "pa":int.from_bytes(data[o+8:o+16],"little"),
                        "rights":data[o+16]})
    return root,used,targets

def cpu_stub_bounds(targets):
    if len(targets)!=COUNT:raise ValueError("missing live descriptor CPU span")
    first,last=targets[25:]
    if (first["rights"]!=1 or last["rights"]!=1 or
        first["va"]!=first["pa"] or last["va"]!=last["pa"] or
        not 0x100000<=first["va"]<1<<32 or
        not first["va"]<last["va"]<1<<32 or
        last["va"]-first["va"]>=1024 or
        (last["va"]>>12)-(first["va"]>>12)>1):
        raise ValueError("live-CPU stub lacks bounded identical supervisor RX span")
    return first["va"],last["va"]+1

def verify_new(root,used,targets,records,old_root,reader,old_reader):
    begin,end=cpu_stub_bounds(targets)
    proof=previous_verify(root,used,targets[:PRIOR],records,old_root,reader,old_reader)
    for p in targets[25:]:
        old=witness_one(old_root,p["va"],old_reader)
        if old["effective_user"] or not old["effective_execute"]:
            raise ValueError("live-CPU code not old-root supervisor executable")
        if walk_new(root,used,p["va"],reader)!=(p["pa"],1):
            raise ValueError("live-CPU code not same-PA supervisor RX under private root")
    proof.update({"new_target_count":COUNT,
        "live_descriptors_begin":f"0x{begin:x}",
        "live_descriptors_end":f"0x{end:x}",
        "live_descriptors_code_old_new_rx":True,
        "kernel_entry_executed":False})
    return proof
