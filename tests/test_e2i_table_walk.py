"""Negative host controls for the E2i independent PML4 table walker."""
import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2i_table_walk import walk_tables, parse_full_page_dump, MAX_TABLES

def page(*entries):
    w=[0]*512
    for i,v in entries:w[i]=v
    return w
ROOT=0x120000
PDPT=0x130000
PD=0x140000
PT=0x150000
def fixture():
    return {
        ROOT:page((0,PDPT|3)),
        PDPT:page((0,PD|3)),
        PD:page((0,PT|3),(1,0x200000|0x83)),
        PT:page((0,0x400000|3))
    }
def dump(addr,words):
    lines=[]
    for i in range(0,512,2):
        lines.append(f"0x{addr+i*8:x}: 0x{words[i]:016x} 0x{words[i+1]:016x}")
    return "\n".join(lines)
class E2iHostWalkTest(unittest.TestCase):
    def test_all_four_levels(self):
        d=fixture()
        result=walk_tables(ROOT,lambda pa:d[pa])
        self.assertEqual(result["table_pages"],4)
        self.assertEqual(result["pte_qwords_verified"],2048)
        self.assertEqual(result["huge_2m_leaves"],1)
        self.assertEqual(result["leaf_4k_pages"],1)
    def test_full_page_dump_accepted(self):
        p=fixture()[ROOT]
        self.assertEqual(parse_full_page_dump(dump(ROOT,p),ROOT),tuple(p))
    def test_truncated_page_fails(self):
        s=dump(ROOT,fixture()[ROOT])
        with self.assertRaisesRegex(ValueError,"truncated"):
            parse_full_page_dump("\n".join(s.splitlines()[:-1]),ROOT)
    def test_duplicate_line_fails(self):
        s=dump(ROOT,fixture()[ROOT])
        with self.assertRaisesRegex(ValueError,"skipped/overlapping"):
            parse_full_page_dump(s+"\n"+s.splitlines()[0],ROOT)
    def test_wrong_dump_address_fails(self):
        with self.assertRaisesRegex(ValueError,"skipped/overlapping"):
            parse_full_page_dump(dump(ROOT+8,fixture()[ROOT]),ROOT)
    def test_corrupted_pte_detected_in_digest(self):
        d=fixture()
        a=walk_tables(ROOT,lambda pa:d[pa])["host_table_digest_sha256"]
        d[PT][0]=0x500003
        b=walk_tables(ROOT,lambda pa:d[pa])["host_table_digest_sha256"]
        self.assertNotEqual(a,b)
    def test_child_missing_fails(self):
        d=fixture()
        with self.assertRaises(KeyError):
            walk_tables(ROOT,lambda pa:d[pa] if pa!=PD else d[0])
    def test_large_page_skips_child_read(self):
        d={ROOT:page((0,PDPT|3)),PDPT:page((0,0x40000000|0x83))}
        self.assertEqual(walk_tables(ROOT,lambda pa:d[pa])["table_pages"],2)
    def test_root_huge_page_forbidden(self):
        d={ROOT:page((0,0x200000|0x83))}
        with self.assertRaisesRegex(ValueError,"PML4 huge"):
            walk_tables(ROOT,lambda pa:d[pa])
    def test_reused_cross_depth_forbidden(self):
        d=fixture();d[PDPT][1]=PDPT|3
        with self.assertRaisesRegex(ValueError,"another depth"):
            walk_tables(ROOT,lambda pa:d[pa])
    def test_bounded_tables(self):
        d=fixture()
        with self.assertRaisesRegex(ValueError,"exceeds bound"):
            walk_tables(ROOT,lambda pa:d[pa],max_tables=2)
    def test_no_leaf_fails(self):
        d={ROOT:page((0,PDPT|3)),PDPT:page()}
        with self.assertRaisesRegex(ValueError,"no present leaves"):
            walk_tables(ROOT,lambda pa:d[pa])
    def test_rejects_unmapped_child(self):
        d=fixture();d[PD][0]=3
        with self.assertRaisesRegex(ValueError,"bad child"):
            walk_tables(ROOT,lambda pa:d[pa])
    def test_out_of_ram_pointer_fails(self):
        d=fixture();d[PD][0]=0x1_0000_0000|3
        with self.assertRaisesRegex(ValueError,"bad child"):
            walk_tables(ROOT,lambda pa:d[pa])
    def test_limit_invalid(self):
        with self.assertRaisesRegex(ValueError,"invalid root/cap"):
            walk_tables(ROOT,lambda pa:page(),max_tables=MAX_TABLES+1)
if __name__=="__main__":unittest.main()
