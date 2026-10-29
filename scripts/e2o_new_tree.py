"""E2o independent paused-QMP old/new CR3 table comparison (new CR3 NOT loaded).

COM2 guest manifest contains *untrusted expectations*: the host separately
reads every traversed new table frame as 4096 physical bytes via QMP HMP,
cross-checks its old-CR3 virtual alias and demands exact PTE rights/PA.
"""
from e2i_table_walk import PAGE, ADDRESS_MASK, PRESENT
from e2j_targeted import witness_one

MARKER=b"\x7fUNI2O\x7f"
COUNT=19
RECORD_LEN=17
PACKET=10+COUNT*RECORD_LEN
BASE=0xffffffff80000000
LIMIT=0xffffffff84000000
MAIN_GUARD=LIMIT-0x20000
MAIN_STACK=MAIN_GUARD+PAGE
IST_GUARD=LIMIT-0x10000
IST_STACK=IST_GUARD+PAGE
DESCRIPTORS=LIMIT-0x40000
CONTEXT=LIMIT-0x30000
RIGHTS={0:"unmapped",1:"rx",2:"rw",3:"ro"}

def decode_manifest(data):
    if not isinstance(data,(bytes,bytearray)) or len(data)!=PACKET:
        raise ValueError("E2o new-private manifest exact size required")
    root=int.from_bytes(data[0:8],"little")
    used=data[8]
    count=data[9]
    if count!=COUNT or not 4<=used<=128 or root<PAGE or root%PAGE or root>=1<<32:
        raise ValueError("E2o bad root, used tables, or target count")
    targets=[]
    for i in range(COUNT):
        offset=10+i*RECORD_LEN
        va=int.from_bytes(data[offset:offset+8],"little")
        pa=int.from_bytes(data[offset+8:offset+16],"little")
        rights=data[offset+16]
        if rights not in RIGHTS:raise ValueError("invalid E2o target rights")
        targets.append({"va":va,"pa":pa,"rights":rights})
    return root,used,targets

def validate_expected(root,used,targets,records,old_root,old_reader):
    """Check guest claims against reservation provenance and fixed L1 VA contract."""
    if len(targets)!=COUNT:raise ValueError("wrong number of private targets")
    aux={r["id"]:r for r in records if r["id"]!=1}
    if set(aux)!={2,3,4,5,6,7,8}:raise ValueError("missing EFI auxiliary reservations")
    if root!=aux[2]["base"] or root==old_root or used>aux[2]["pages"]:
        raise ValueError("new PML4 root outside owned exact page-table reservation")
    # Distinct code pointers inside EFI low 4GiB; independent old-root walk
    # must affirm identity, kernel-supervisor and executable. EFI descriptor
    # ownership remains guest-audited, not independently authenticated here.
    for i in range(1,5):
        t=targets[i]
        if (not 0x100000<=t["va"]<1<<32 or
            t["pa"]!=t["va"] or t["rights"]!=1):
            raise ValueError("untrusted EFI executable code anchor")
        old=witness_one(old_root,t["va"],old_reader)
        if old["effective_user"] or not old["effective_execute"]:
            raise ValueError("EFI code anchor not old-root supervisor executable")
    if not targets[1]["va"]<targets[2]["va"] or \
       targets[2]["va"]-targets[1]["va"]>=1024:
        raise ValueError("bridge span not finite")
    entry=targets[0]
    if (not BASE<=entry["va"]<LIMIT or
        entry["rights"]!=1 or
        not any(r["id"]==1 and r["base"]<=entry["pa"]<r["end"] for r in records)):
        raise ValueError("kernel RIP not owned staged RX segment page")
    expected=[]
    for j in range(2):
        va=aux[3]["base"]+j*PAGE
        expected.append((va,va,2))
    for j in range(4):
        expected.append((MAIN_STACK+j*PAGE,aux[4]["base"]+j*PAGE,2))
    for j in range(4):
        expected.append((IST_STACK+j*PAGE,aux[5]["base"]+j*PAGE,2))
    expected.extend([
        (DESCRIPTORS,aux[6]["base"],2),
        (CONTEXT,aux[7]["base"],3),
        (MAIN_GUARD,0,0),(IST_GUARD,0,0)])
    if [(t["va"],t["pa"],t["rights"]) for t in targets[5:]]!=expected:
        raise ValueError("new PML4 stack/metadata/guard claims violate contract")
    return aux

def walk_new(root,used,va,reader):
    """Explicit four-level 4KiB PTE decode; NO active CR3 translation."""
    canonical=(va>>48)==(0xffff if va&(1<<47) else 0)
    if not canonical:raise ValueError("noncanonical new-root target")
    table=root
    parent_write=True
    parent_exec=True
    for level,shift in enumerate((39,30,21,12)):
        words=reader(table)
        if len(words)!=512:raise ValueError("missing full QMP table")
        entry=words[(va>>shift)&511]
        if not entry&PRESENT:
            if entry!=0:raise ValueError("nonzero nonpresent new PTE")
            return None
        if entry&4:raise ValueError("new root user-accessible PTE")
        if entry&(1<<7):raise ValueError("new-root huge page forbidden")
        parent_write &= bool(entry&2)
        parent_exec &= not bool(entry&(1<<63))
        if level==3:
            physical=(entry&ADDRESS_MASK)|(va&(PAGE-1))
            if entry&2:
                if not entry&(1<<63) or not parent_write:
                    raise ValueError("new-root W+X or nonwritable parent")
                rights=2
            elif entry&(1<<63):
                rights=3
            else:
                if not parent_exec:raise ValueError("RX path NX")
                rights=1
            if rights==1 and not parent_exec:
                raise ValueError("new-root RX ancestor is NX")
            return physical,rights
        if not entry&2 or entry&(1<<63):
            raise ValueError("new root intermediate table restrictions")
        child=entry&ADDRESS_MASK
        if child<root or child>=root+used*PAGE or (child-root)%PAGE:
            raise ValueError("new private child escapes allocated table frames")
        table=child
    raise AssertionError("unreachable")

def verify_new(root,used,targets,records,old_root,reader,old_reader):
    validate_expected(root,used,targets,records,old_root,old_reader)
    cache={}
    def cached(pa):
        if pa<root or pa>=root+used*PAGE or (pa-root)%PAGE:
            raise ValueError("new root table outside allocated frames")
        if pa not in cache:
            if len(cache)>=128:raise ValueError("new root table-frame cap")
            cache[pa]=tuple(reader(pa))
        return cache[pa]
    checked=0
    for item in targets:
        got=walk_new(root,used,item["va"],cached)
        if item["rights"]==0:
            if got is not None:raise ValueError("new private guard wrongly mapped")
        elif got!=(item["pa"],item["rights"]):
            raise ValueError("new private PTE PA or access rights mismatch")
        checked+=1
    # Independently audit *all used table frames*, not just path samples.
    for pa in range(root,root+used*PAGE,PAGE):
        cached(pa)
    return {"new_private_cr3_host_independently_audited":True,
            "new_root_pa":f"0x{root:x}",
            "new_table_pages_compared":len(cache),
            "new_target_count":checked,
            "new_unmapped_guards":2,
            "new_root_activated":False}
