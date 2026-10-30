import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from audit_e2r_high_ud_trace import verify_native_high_ud

BASE=b"ffffffff800002c0"
VALID=b"0: v=06 e=0000 i=0 cpl=0 IP=0008:"+BASE+b" pc=0000000000000000 SP=0010:ffffffff83fe4f88\n"

class KernelOriginUd(unittest.TestCase):
    def test_native_high_half_event(self):
        r=verify_native_high_ud(VALID)
        self.assertEqual(r["native_vector"],6)
        self.assertEqual(r["native_high_half_ud_events"],1)
    def test_wrong_vector(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID.replace(b"v=06",b"v=0d"))
    def test_software_interrupt(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID.replace(b"i=0",b"i=1"))
    def test_old_firmware_selector(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID.replace(b"IP=0008",b"IP=0038"))
    def test_low_firmware_rip(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID.replace(BASE,b"000000000dfdc000"))
    def test_wrong_cpl(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID.replace(b"cpl=0",b"cpl=3"))
    def test_duplicate_ud(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID+VALID)
    def test_qemu_reset(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID+b"\nTriple fault\n")
    def test_empty(self):
        with self.assertRaises(ValueError):verify_native_high_ud(b"")
    def test_wrong_elf_entry(self):
        with self.assertRaises(ValueError):verify_native_high_ud(VALID,0xffffffff80001000)
if __name__=="__main__":unittest.main()
