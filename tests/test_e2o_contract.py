import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2o_new_tree import (
    verify_new,COUNT,BASE,LIMIT,MAIN_STACK,IST_STACK,
    MAIN_GUARD,IST_GUARD,DESCRIPTORS,CONTEXT,PAGE)
ROOT=0x02000000
OLD=0x01000000
def fixture():
    records=[{"id":1,"base":0x08000000,"end":0x08003000,"pages":3}]
    for ident,base,pages in [
        (2,ROOT,128),(3,0x03000000,2),(4,0x04000000,4),
        (5,0x05000000,4),(6,0x06000000,1),
        (7,0x07000000,1),(8,0x09000000,1)]:
        records.append({"id":ident,"base":base,"end":base+pages*PAGE,"pages":pages})
    addresses=[
        (BASE+0x123,0x08000123,1),
        (0x15100010,0x15100010,1),
        (0x15100013,0x15100013,1),
        (0x15200000,0x15200000,1),
        (0x15300000,0x15300000,1)]
    for n in range(2):
        addresses.append((0x03000000+n*PAGE,0x03000000+n*PAGE,2))
    for n in range(4):
        addresses.append((MAIN_STACK+n*PAGE,0x04000000+n*PAGE,2))
    for n in range(4):
        addresses.append((IST_STACK+n*PAGE,0x05000000+n*PAGE,2))
    addresses.extend([
        (DESCRIPTORS,0x06000000,2),(CONTEXT,0x07000000,3),
        (MAIN_GUARD,0,0),(IST_GUARD,0,0)])
    assert len(addresses)==COUNT
    targets=[{"va":v,"pa":p,"rights":r} for v,p,r in addresses]
    new={ROOT:[0]*512}
    def map_leaf(va,pa,r):
        cur=ROOT
        for shift in (39,30,21):
            idx=(va>>shift)&511
            entry=new[cur][idx]
            if entry==0:
                next_pa=ROOT+len(new)*PAGE
                new[next_pa]=[0]*512
                new[cur][idx]=next_pa|3
                cur=next_pa
            else:
                cur=entry&0x000f_ffff_ffff_f000
        idx=(va>>12)&511
        bits={1:1,2:3|(1<<63),3:1|(1<<63)}[r]
        word=(pa&~(PAGE-1))|bits
        if new[cur][idx] not in (0,word):
            raise AssertionError("conflicting leaf")
        new[cur][idx]=word
    for va,pa,rights in addresses:
        if rights:
            map_leaf(va,pa,rights)
    old={
        OLD:[0]*512,
        OLD+PAGE:[0]*512,
        OLD+2*PAGE:[0]*512,
    }
    old[OLD][0]=OLD+PAGE|3
    old[OLD+PAGE][0]=OLD+2*PAGE|3
    for code in addresses[1:5]:
        va=code[0]
        old[OLD+2*PAGE][(va>>21)&511]=(va&~((1<<21)-1))|0x83
    return new,targets,records,old
class HostPrivateTreeIntegration(unittest.TestCase):
    def check(self,new,targets,records,old,root=ROOT):
        return verify_new(root,len(new),targets,records,OLD,
                          lambda pa:new[pa],lambda pa:old[pa])
    def test_actual_end_to_end_target_matrix(self):
        n,t,r,o=fixture()
        result=self.check(n,t,r,o)
        self.assertTrue(result["new_private_cr3_host_independently_audited"])
        self.assertEqual(result["new_target_count"],19)
        self.assertEqual(result["new_table_pages_compared"],len(n))
        self.assertFalse(result["new_root_activated"])
    def test_spoofed_guest_root_rejected(self):
        n,t,r,o=fixture()
        with self.assertRaisesRegex(ValueError,"root outside"):
            self.check(n,t,r,o,root=ROOT+PAGE)
    def test_spoofed_stack_claim_rejected(self):
        n,t,r,o=fixture()
        t[7]["pa"]+=PAGE
        with self.assertRaisesRegex(ValueError,"contract"):
            self.check(n,t,r,o)
    def test_spoofed_executable_claim_rejected(self):
        n,t,r,o=fixture()
        t[1]["rights"]=2
        with self.assertRaisesRegex(ValueError,"executable"):
            self.check(n,t,r,o)
    def test_changed_pte_physical_rejected(self):
        n,t,r,o=fixture()
        leaf_va=MAIN_STACK
        for words in n.values():
            idx=(leaf_va>>12)&511
            word=words[idx]
            if word&0x000f_ffff_ffff_f000==0x04000000 and word>>63:
                words[idx]=0x04100000|(word&((1<<64)-1)&~0x000f_ffff_ffff_f000)
                break
        with self.assertRaisesRegex(ValueError,"mismatch"):
            self.check(n,t,r,o)
    def test_host_rejects_present_guard(self):
        n,t,r,o=fixture()
        # Add one leaf mapping to a formerly absent guard, without
        # changing any claimed metadata.
        cur=ROOT
        for shift in (39,30,21):
            idx=(MAIN_GUARD>>shift)&511
            cur=n[cur][idx]&0x000f_ffff_ffff_f000
        n[cur][(MAIN_GUARD>>12)&511]=0x0a000003|(1<<63)
        with self.assertRaisesRegex(ValueError,"guard"):
            self.check(n,t,r,o)
    def test_old_root_code_path_must_be_executable(self):
        n,t,r,o=fixture()
        idx=(t[1]["va"]>>21)&511
        o[OLD+2*PAGE][idx]|=1<<63
        with self.assertRaisesRegex(ValueError,"not old-root"):
            self.check(n,t,r,o)
if __name__=="__main__":unittest.main()
