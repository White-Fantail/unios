import unittest
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from audit_e2k_pe import audit,Reject,EXPECTED
def pe_fixture():
    d=bytearray(0x600)
    d[:2]=b"MZ";d[0x3c:0x40]=(0x80).to_bytes(4,"little")
    d[0x80:0x84]=b"PE\0\0"
    d[0x84:0x86]=(0x8664).to_bytes(2,"little")
    d[0x86:0x88]=(1).to_bytes(2,"little")
    d[0x94:0x96]=(0xf0).to_bytes(2,"little")
    d[0x98:0x9a]=(0x20b).to_bytes(2,"little")
    p=0x188
    d[p:p+8]=b".text\0\0\0"
    d[p+16:p+20]=(0x200).to_bytes(4,"little")
    d[p+20:p+24]=(0x200).to_bytes(4,"little")
    d[p+36:p+40]=(0x60000020).to_bytes(4,"little")
    d[0x220:0x22b]=EXPECTED["generic"]
    d[0x240:0x24b]=EXPECTED["double_fault"]
    return d
class PeEmergencyAudit(unittest.TestCase):
    def test_fixture(self):self.assertEqual(len(audit(pe_fixture())),2)
    def test_bad_magic(self):
        d=pe_fixture();d[0]=0
        with self.assertRaises(Reject):audit(d)
    def test_corrupt_code(self):
        d=pe_fixture();d[0x220+8]=0x90
        with self.assertRaises(Reject):audit(d)
    def test_duplicate_handler(self):
        d=pe_fixture();d[0x280:0x28b]=EXPECTED["generic"]
        with self.assertRaises(Reject):audit(d)
    def test_writable_executable_section(self):
        d=pe_fixture();d[0x188+39]|=0x80
        with self.assertRaises(Reject):audit(d)
    def test_not_pe32_plus(self):
        d=pe_fixture();d[0x99]=0x00
        with self.assertRaises(Reject):audit(d)
    def test_truncated_section(self):
        with self.assertRaises(Reject):audit(pe_fixture()[:0x300])
if __name__=="__main__":unittest.main()
