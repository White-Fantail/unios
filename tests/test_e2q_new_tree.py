import unittest,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2q_new_tree import parse_switched_regs,decode_manifest,PACKET,COUNT
def regs(cr3=0xdf5e000,rip=0x15100020,rsp=0x03002000):
    return f"RAX=0000000000000000\nRSP={rsp:016x} RIP={rip:016x}\nCR3={cr3:016x}\n"
class ThirdHostBarrier(unittest.TestCase):
    def test_real_cr3_rip_rsp_bounds(self):
        p=parse_switched_regs(regs(),0xdf5e000,0x15100000,0x15100100,0x03000000,0x03002000)
        self.assertTrue(p["cpu_private_cr3_activated"])
        self.assertFalse(p["kernel_entry_executed"])
    def test_old_cr3_rejected(self):
        with self.assertRaisesRegex(ValueError,"never loaded"):
            parse_switched_regs(regs(cr3=0xf801000),0xdf5e000,0x15100000,0x15100100,0x03000000,0x03002000)
    def test_unmapped_rip_rejected(self):
        with self.assertRaisesRegex(ValueError,"RIP escaped"):
            parse_switched_regs(regs(rip=0x16100000),0xdf5e000,0x15100000,0x15100100,0x03000000,0x03002000)
    def test_wrong_rsp_rejected(self):
        with self.assertRaisesRegex(ValueError,"RSP not"):
            parse_switched_regs(regs(rsp=0x0),0xdf5e000,0x15100000,0x15100100,0x03000000,0x03002000)
    def test_unaligned_rsp_rejected(self):
        with self.assertRaisesRegex(ValueError,"RSP not"):
            parse_switched_regs(regs(rsp=0x03001fff),0xdf5e000,0x15100000,0x15100100,0x03000000,0x03002000)
    def test_extra_regs_refused(self):
        with self.assertRaisesRegex(ValueError,"duplicate"):
            parse_switched_regs(regs()+"CR3=000000000df5e000\n",0xdf5e000,0x15100000,0x15100100,0x03000000,0x03002000)
    def test_extended_manifest_size_fail_closed(self):
        self.assertEqual(PACKET,10+COUNT*17)
        with self.assertRaises(ValueError):decode_manifest(b"\0"*PACKET)
        with self.assertRaises(ValueError):decode_manifest(b"\0"*(PACKET-17))
if __name__=="__main__":unittest.main()
