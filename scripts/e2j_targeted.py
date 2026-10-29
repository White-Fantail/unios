"""E2j: targeted, bounded live old-CR3 leaf witness for EFI reservations.

This is pinned QEMU/OVMF-only pre-dereference evidence. Neither a
portable firmware memory proof nor a new-CR3/ExitBootServices proof.
"""
import re
from e2i_table_walk import ADDRESS_MASK, PAGE, PRESENT, PS, qmp_read_table

MARKER=b"\x7fUNI2J\x7f"
MAX_RECORDS=16
ALLOWED_AUX={2:128,3:2,4:4,5:4,6:1,7:1,8:1}
MAX_WITNESS_TARGETS=36

def parse_manifest(raw):
    """Exact untrusted COM2 manifest: byte count, then (id:u8,pages:u32,pa:u64)."""
    if not isinstance(raw,(bytes,bytearray)) or len(raw)<1:
        raise ValueError("empty target manifest")
    count=raw[0]
    if count<8 or count>MAX_RECORDS or len(raw)!=1+13*count:
        raise ValueError("bad bounded target manifest length")
    records=[]
    last_id=1
    first_aux=0
    for i in range(count):
        off=1+13*i
        rid=raw[off]
        pages=int.from_bytes(raw[off+1:off+5],"little")
        base=int.from_bytes(raw[off+5:off+13],"little")
        if rid==1:
            if first_aux: raise ValueError("segments must precede auxiliary pages")
            if not 1<=pages<=4096: raise ValueError("invalid segment extent")
        else:
            first_aux+=1
            if rid != last_id+1 or ALLOWED_AUX.get(rid)!=pages:
                raise ValueError("auxiliary reservation order or size mismatch")
            last_id=rid
        if base<PAGE or base%PAGE or base+pages*PAGE>1<<32:
            raise ValueError("reservation outside pinned 4GiB RAM window")
        end=base+pages*PAGE
        if any(base<o["end"] and o["base"]<end for o in records):
            raise ValueError("overlapping EFI allocated reservations")
        records.append({"id":rid,"base":base,"end":end,"pages":pages})
    if first_aux!=7 or count-first_aux>8:
        raise ValueError("missing fixed EFI auxiliary reservation")
    return records

def manifest_targets(records):
    """First+last page of each EFI reservation; bounded, not an exhaustive map."""
    targets=set()
    for r in records:
        targets.add(r["base"])
        targets.add(r["end"]-PAGE)
    if len(targets)>MAX_WITNESS_TARGETS:
        raise ValueError("too many lower-leaf path targets")
    return sorted(targets)

def witness_one(root,va,reader):
    """Decode all traversed PML4/PDPT/PD/PT frames, handle 1G/2M pages."""
    if root<PAGE or root&(PAGE-1) or root>=1<<32 or va<PAGE or va>=1<<32:
        raise ValueError("out-of-profile physical/linear address")
    table=root
    visited=[]
    effective_write=True
    effective_user=True
    effective_exec=True
    for depth,shift in enumerate((39,30,21,12)):
        words=reader(table)
        if len(words)!=512:
            raise ValueError("incomplete table page")
        idx=(va>>shift)&511
        pte=words[idx]
        if not pte&PRESENT: raise ValueError(f"unmapped target at level {depth}")
        effective_write &= bool(pte&2)
        effective_user &= bool(pte&4)
        effective_exec &= not bool(pte&(1<<63))
        visited.append(table)
        if depth==0 and pte&PS:
            raise ValueError("invalid PML4 huge-page entry")
        if depth in (1,2) and pte&PS:
            size=1<<shift
            pa=(pte&ADDRESS_MASK)&~(size-1)
            result=pa|(va&(size-1))
            if result!=va: raise ValueError("old-CR3 large-page translation not identity")
            return {"va":va,"pa":result,"page_size":size,"frames":visited,
                    "effective_write":effective_write,"effective_user":effective_user,
                    "effective_execute":effective_exec}
        if depth==3:
            result=(pte&ADDRESS_MASK)|(va&(PAGE-1))
            if result!=va: raise ValueError("old-CR3 4K translation not identity")
            return {"va":va,"pa":result,"page_size":PAGE,"frames":visited,
                    "effective_write":effective_write,"effective_user":effective_user,
                    "effective_execute":effective_exec}
        table=pte&ADDRESS_MASK
        if table<PAGE or table%PAGE or table>=1<<32:
            raise ValueError("next-level page table address invalid")
    raise ValueError("unreachable")

def verify_targets(root,records,qmp,read_table=None):
    """Host provides readings, independent of guest's claimed translations."""
    from e2h_root_host import parse_translation
    read_table=read_table or (lambda pa:qmp_read_table(qmp,pa))
    cache={}
    def cached(pa):
        if pa not in cache:
            if len(cache)>=64: raise ValueError("targeted table frame cap exceeded")
            cache[pa]=tuple(read_table(pa))
        return cache[pa]
    targets=manifest_targets(records)
    checked=[]
    size_counts={"1G":0,"2M":0,"4K":0}
    for va in targets:
        outcome=witness_one(root,va,cached)
        if not outcome["effective_write"]:
            raise ValueError("EFI reservation not writable under old CR3")
        parse_translation(qmp.hmp(f"gva2gpa 0x{va:x}"),va)
        checked.append(outcome)
        name={1<<30:"1G",1<<21:"2M",PAGE:"4K"}.get(outcome["page_size"])
        if name is None:raise ValueError("unknown leaf size")
        size_counts[name]+=1
    return {
        "reservation_count":len(records),
        "sampled_reservation_pages":len(targets),
        "sampled_lower_table_frames":max(0,len(cache)-1),
        "sampled_leaf_sizes":size_counts,
        "lower_pd_pt_frames_observed":any(len(item["frames"])>=3 for item in checked),
        "fully_compared_table_pages":len(cache),
        "verified_pte_qwords":len(cache)*512,
        "sampled_reservation_ends_only":True,
        "full_reserved_page_coverage":False,
        "scope":"first and last page of each EFI reservation; no intermediate leaf assertion",
    }
