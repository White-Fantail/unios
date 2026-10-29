import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2o_new_tree import (
    decode_manifest,verify_new,walk_new,COUNT,PACKET,MAIN_STACK,
    MAIN_GUARD,IST_GUARD,BASE,PAGE)
from e2j_targeted import witness_one
ROOT=0x02000000
def packet(root=ROOT,used=4):
    return root.to_bytes(8,"little")+bytes([used,COUNT])+b"\0"*(COUNT*17)
def table_fixture():
    tables={ROOT:[0]*512,ROOT+PAGE:[0]*512,
            ROOT+2*PAGE:[0]*512,ROOT+3*PAGE:[0]*512}
    va=MAIN_STACK
    current=ROOT
    for depth,shift in enumerate((39,30,21)):
        tables[current][(va>>shift)&511]=(ROOT+(depth+1)*PAGE)|3
        current=ROOT+(depth+1)*PAGE
    tables[current][(va>>12)&511]=0x03000000|3|(1<<63)
    return tables
class E2o(unittest.TestCase):
    def test_packet_length(self):
        self.assertEqual(len(packet()),PACKET)
        self.assertEqual(decode_manifest(packet())[1],4)
    def test_wrong_length(self):
        with self.assertRaises(ValueError):decode_manifest(packet()[:-1])
    def test_illegal_target_count(self):
        p=bytearray(packet());p[9]=18
        with self.assertRaises(ValueError):decode_manifest(p)
    def test_unaligned_root(self):
        with self.assertRaises(ValueError):decode_manifest(packet(ROOT+1))
    def test_positive_new_rw_nx(self):
        d=table_fixture()
        self.assertEqual(walk_new(ROOT,4,MAIN_STACK,lambda pa:d[pa]),(0x03000000,2))
    def test_guard_missing(self):
        d=table_fixture()
        self.assertIsNone(walk_new(ROOT,4,IST_GUARD,lambda pa:d[pa]))
    def test_corrupt_wx_leaf(self):
        d=table_fixture()
        d[ROOT+3*PAGE][(MAIN_STACK>>12)&511]&=~(1<<63)
        with self.assertRaisesRegex(ValueError,"W\\+X"):
            walk_new(ROOT,4,MAIN_STACK,lambda pa:d[pa])
    def test_corrupt_parent(self):
        d=table_fixture()
        d[ROOT][(MAIN_STACK>>39)&511]=0x99990003
        with self.assertRaisesRegex(ValueError,"escapes"):
            walk_new(ROOT,4,MAIN_STACK,lambda pa:d[pa])
    def test_user_leaf(self):
        d=table_fixture()
        d[ROOT+3*PAGE][(MAIN_STACK>>12)&511]|=4
        with self.assertRaisesRegex(ValueError,"user"):
            walk_new(ROOT,4,MAIN_STACK,lambda pa:d[pa])
    def test_forbidden_huge(self):
        d=table_fixture()
        d[ROOT+PAGE][(MAIN_STACK>>30)&511]|=0x80
        with self.assertRaisesRegex(ValueError,"huge"):
            walk_new(ROOT,4,MAIN_STACK,lambda pa:d[pa])
    def test_truncated_physical_frame(self):
        d=table_fixture();d[ROOT]=d[ROOT][:-1]
        with self.assertRaisesRegex(ValueError,"full"):
            walk_new(ROOT,4,MAIN_STACK,lambda pa:d[pa])
if __name__=="__main__":unittest.main()
