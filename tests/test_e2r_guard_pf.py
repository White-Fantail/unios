import unittest,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from audit_e2r_guard_pf_trace import verify_native_high_guard_pf,BASE
from e2o_new_tree import MAIN_GUARD
VALID=(f"0: v=0e e=0000 i=0 cpl=0 IP=0008:{BASE+0x152:016x} "
       "pc=0000000000000000\n"
       "RAX=0000000000000000 RSP=ffffffff83fe4f88\n"
       f"CR0=80010033 CR2={MAIN_GUARD:016x} CR3=000000000df5c000\n").encode()
class NativeGuardFault(unittest.TestCase):
    def check_bad(self,raw):
        with self.assertRaises(ValueError):verify_native_high_guard_pf(raw)
    def test_valid(self):
        x=verify_native_high_guard_pf(VALID)
        self.assertEqual(x["native_vector"],14)
        self.assertEqual(x["native_fault_error"],0)
        self.assertEqual(x["native_cr2"],hex(MAIN_GUARD))
    def test_wrong_vector(self):self.check_bad(VALID.replace(b"v=0e",b"v=06"))
    def test_wrong_code_selector(self):self.check_bad(VALID.replace(b"IP=0008:",b"IP=0038:"))
    def test_low_rip(self):self.check_bad(VALID.replace(f"{BASE+0x152:016x}".encode(),b"000000000dfdc000"))
    def test_wrong_cr2(self):self.check_bad(VALID.replace(f"{MAIN_GUARD:016x}".encode(),b"ffffffff83ff0000"))
    def test_write_fault(self):self.check_bad(VALID.replace(b"e=0000",b"e=0002"))
    def test_protection_fault(self):self.check_bad(VALID.replace(b"e=0000",b"e=0001"))
    def test_duplicate(self):self.check_bad(VALID+VALID)
    def test_software_fault(self):self.check_bad(VALID.replace(b"i=0",b"i=1"))
    def test_firmware_cpl(self):self.check_bad(VALID.replace(b"cpl=0",b"cpl=3"))
    def test_reset(self):self.check_bad(VALID+b"CPU reset\n")
if __name__=="__main__":unittest.main()
