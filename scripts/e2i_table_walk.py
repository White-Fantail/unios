"""LAB-H1 E2i: host-only four-level firmware page-table witness.

Read all reachable *page-table frames* while CPU0 is paused. QMP HMP
gva2gpa, xp and x are independent observations of the guest's own
page-table walking; do not infer identity from any guest-reported data.
This only works on the pinned disposable QEMU/OVMF profile, NOT hardware.
"""
import hashlib
import re
import struct

PAGE=4096
TABLE_ENTRIES=512
PRESENT=1
PS=1<<7
ADDRESS_MASK=0x000f_ffff_ffff_f000
MAX_TABLES=256

def parse_full_page_dump(response, physical):
    """Require exact sequential QEMU HMP memory output, never a sparse sample."""
    items=[]
    if not isinstance(response,str): raise ValueError("bad QEMU memory dump type")
    for line in response.strip().splitlines():
        m=re.fullmatch(r"\s*(0x[0-9a-fA-F]+):\s*(.*?)\s*",line)
        if not m: raise ValueError(f"unexpected HMP memory line {line[:200]!r}; dump head {response[:250]!r}")
        offset=int(m.group(1),16)
        if offset!=physical+8*len(items):
            raise ValueError("QEMU page dump has skipped/overlapping addresses")
        words=re.findall(r"0x[0-9a-fA-F]{1,16}\b",m.group(2))
        if not words or " ".join(words)!=m.group(2).strip():
            raise ValueError("invalid HMP PTE words")
        items.extend(int(w,16) for w in words)
        if len(items)>TABLE_ENTRIES:
            raise ValueError("QEMU returned excess PTE words")
    if len(items)!=TABLE_ENTRIES:
        raise ValueError(f"QEMU page is truncated: {len(items)}/512 qwords")
    return tuple(items)

def walk_tables(root, read_table, max_tables=MAX_TABLES):
    """BFS of every reachable page-table PA; skip hardware 1G/2M leaves."""
    if not 0<max_tables<=MAX_TABLES or root==0 or root&(PAGE-1):
        raise ValueError("invalid root/cap")
    stack=[(root,0)]
    visited={}
    digest=hashlib.sha256()
    leaves_1g=leaves_2m=leaves_4k=0
    while stack:
        pa,level=stack.pop()
        if pa in visited:
            if visited[pa]!=level:
                raise ValueError("physical page-table frame reused at another depth")
            continue
        if len(visited)>=max_tables: raise ValueError("page table count exceeds bound")
        if pa&(PAGE-1) or pa==0 or pa>=1<<32:
            raise ValueError("firmware table frame outside pinned guest physical RAM")
        words=tuple(read_table(pa))
        if len(words)!=TABLE_ENTRIES or any(not isinstance(x,int) or x<0 or x>=1<<64 for x in words):
            raise ValueError("bad complete page table")
        visited[pa]=level
        digest.update(struct.pack("<QB",pa,level))
        digest.update(struct.pack("<512Q",*words))
        for pte in words:
            if not pte&PRESENT: continue
            if level==3:
                leaves_4k+=1
                continue
            if pte&PS:
                if level==0: raise ValueError("unsupported PML4 huge-page flag")
                if level==1: leaves_1g+=1
                elif level==2: leaves_2m+=1
                continue
            child=pte&ADDRESS_MASK
            if child==0 or child >=1<<32:
                raise ValueError("bad child page-table physical address")
            stack.append((child,level+1))
    if not visited or not (leaves_1g+leaves_2m+leaves_4k):
        raise ValueError("firmware table walk has no present leaves")
    return {
        "table_pages":len(visited),
        "pte_qwords_verified":len(visited)*TABLE_ENTRIES,
        "huge_1g_leaves":leaves_1g,
        "huge_2m_leaves":leaves_2m,
        "leaf_4k_pages":leaves_4k,
        "host_table_digest_sha256":digest.hexdigest(),
        "corroboration":"all reachable table pages PA=VA and full xp/x page bytes agree"
    }

def qmp_read_table(qmp,pa):
    # On pinned HMP 8.x, gva2gpa prints either "gpa: 0x..." or
    # "gva 0x... -> gpa 0x..."; use the reviewed E2h parser.
    from e2h_root_host import parse_translation
    parse_translation(qmp.hmp(f"gva2gpa 0x{pa:x}"),pa)
    physical=parse_full_page_dump(qmp.hmp(f"xp /512gx 0x{pa:x}"),pa)
    virtual=parse_full_page_dump(qmp.hmp(f"x /512gx 0x{pa:x}"),pa)
    if physical!=virtual:
        raise ValueError(f"QMP physical and virtual PTE frames disagree at 0x{pa:x}")
    return physical
