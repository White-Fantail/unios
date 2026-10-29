import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2j_targeted import parse_manifest,manifest_targets,witness_one,verify_targets
from e2i_table_walk import PAGE
ROOT=0x120000;PDPT=0x130000;PD=0x140000;PT=0x150000
def page(*pairs):
    a=[0]*512
    for i,v in pairs:a[i]=v
    return a
def fixture():
    return {ROOT:page((0,PDPT|3)),
            PDPT:page((0,PD|3)),
            PD:page((2,PT|3)),
            PT:page((256,0x500000|3),(257,0x501000|3))}
def encode(items):
    b=bytes((len(items),))
    for rid,pages,base in items:
        b+=bytes((rid,))+pages.to_bytes(4,"little")+base.to_bytes(8,"little")
    return b
AUX=[(2,128,0x1000000),(3,2,0x1100000),(4,4,0x1200000),
     (5,4,0x1300000),(6,1,0x1400000),(7,1,0x1500000),
     (8,1,0x1600000)]
class TargetWitnessTests(unittest.TestCase):
    def test_manifest(self):
        r=parse_manifest(encode([(1,1,0x300000),*AUX]))
        self.assertEqual(len(r),8)
        self.assertEqual(len(manifest_targets(r)),12)
    def test_truncated_manifest_rejected(self):
        with self.assertRaisesRegex(ValueError,"length"):
            parse_manifest(encode([(1,1,0x300000),*AUX])[:-1])
    def test_bad_aux_count(self):
        with self.assertRaisesRegex(ValueError,"length|missing"):
            parse_manifest(encode([(1,1,0x300000),*AUX[:-1]]))
    def test_bad_aux_order(self):
        x=[(1,1,0x300000),*AUX];x[2]=(5,2,0x1100000)
        with self.assertRaisesRegex(ValueError,"order"):
            parse_manifest(encode(x))
    def test_overlap_rejected(self):
        x=[(1,1,0x1000000),*AUX]
        with self.assertRaisesRegex(ValueError,"overlapping"):
            parse_manifest(encode(x))
    def test_outside_ram_rejected(self):
        x=[(1,1,0x1_0000_0000),*AUX]
        with self.assertRaisesRegex(ValueError,"outside"):
            parse_manifest(encode(x))
    def test_4k_path(self):
        d=fixture()
        got=witness_one(ROOT,0x500000,lambda pa:d[pa])
        self.assertEqual(got["page_size"],4096)
        self.assertEqual(len(got["frames"]),4)
    def test_large_page_2m(self):
        d=fixture()
        d[PD][2]=0x400000|0x83
        self.assertEqual(witness_one(ROOT,0x401000,lambda pa:d[pa])["page_size"],2*1024*1024)
    def test_large_page_1g(self):
        d=fixture()
        d[PDPT][1]=0x40000000|0x83
        self.assertEqual(witness_one(ROOT,0x40001000,lambda pa:d[pa])["page_size"],1<<30)
    def test_nonidentity_4k(self):
        d=fixture()
        d[PT][256]=0x502003
        with self.assertRaisesRegex(ValueError,"not identity"):
            witness_one(ROOT,0x500000,lambda pa:d[pa])
    def test_nonidentity_2m(self):
        d=fixture()
        d[PD][2]=0x600000|0x83
        with self.assertRaisesRegex(ValueError,"not identity"):
            witness_one(ROOT,0x401000,lambda pa:d[pa])
    def test_nonpresent(self):
        d=fixture();d[PDPT][0]=0
        with self.assertRaisesRegex(ValueError,"unmapped"):
            witness_one(ROOT,0x500000,lambda pa:d[pa])
    def test_bad_pointer(self):
        d=fixture();d[PDPT][0]=3
        with self.assertRaisesRegex(ValueError,"next-level"):
            witness_one(ROOT,0x500000,lambda pa:d[pa])
    def test_pml4_huge(self):
        d=fixture();d[ROOT][0]|=0x80
        with self.assertRaisesRegex(ValueError,"PML4"):
            witness_one(ROOT,0x500000,lambda pa:d[pa])
    def test_source_read_only(self):
        d=fixture();d[PT][256]=0x500001
        got=witness_one(ROOT,0x500000,lambda pa:d[pa])
        self.assertFalse(got["effective_write"])
    def test_sample_scope_is_not_full_coverage(self):
        records=parse_manifest(encode([(1,1,0x300000),*AUX]))
        self.assertLess(len(manifest_targets(records)),sum(r["pages"] for r in records))
    def test_no_full_coverage_claim(self):
        r=parse_manifest(encode([(1,1,0x300000),*AUX]))
        self.assertGreater(len(manifest_targets(r)),8)
if __name__=="__main__":unittest.main()
