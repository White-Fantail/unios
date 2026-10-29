import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2h_root_host import parse_cr3,parse_translation,parse_words,verify_witness

ROOT=0x123000
REG="RAX=0000000000000000\nCR0=80000001 CR2=0000000000000000 CR3=0000000000123000 CR4=0000000000000020\n"
TR="gva 0x123000 -> gpa 0x123000\n"
PHY="0000000000123000: 0x0000000000456003 0x0000000000000000 0x0000000000000000 0x0000000000000000\n0000000000123020: 0x0000000000000000 0x0000000000000000 0x0000000000000000 0x0000000000000000\n"
class E2hHostWitnessTests(unittest.TestCase):
    def test_parses_registers(self):
        self.assertEqual(parse_cr3(REG),(ROOT,ROOT))
    def test_reports_root_mapping(self):
        self.assertTrue(verify_witness(REG,TR,PHY,PHY,REG)["root_phys_virtual_equal"])
    def test_qemu8_gpa_only_success(self):
        self.assertTrue(parse_translation("gpa: 0x123000\\r\\n",ROOT))
    def test_qemu8_gpa_only_wrong(self):
        with self.assertRaises(ValueError):
            parse_translation("gpa: 0x124000",ROOT)
    def test_rejects_nonidentity(self):
        with self.assertRaises(ValueError):
            verify_witness(REG,"gva 0x123000 -> gpa 0x124000",PHY,PHY,REG)
    def test_rejects_changed_root(self):
        with self.assertRaises(ValueError):
            verify_witness(REG,TR,PHY,PHY,REG.replace("00123000","00124000"))
    def test_rejects_differing_physical_bytes(self):
        with self.assertRaises(ValueError):
            verify_witness(REG,TR,PHY,PHY.replace("00456003","00457003"),REG)
    def test_rejects_no_pml4_bytes(self):
        with self.assertRaises(ValueError):
            parse_words(PHY.replace("00456003","00000000"),ROOT)
    def test_rejects_unmapped(self):
        with self.assertRaises(ValueError):
            parse_translation("gva 0x123000 not mapped",ROOT)
    def test_rejects_missing_cr3(self):
        with self.assertRaises(ValueError):
            parse_cr3("RAX=0 CR0=80000001")
    def test_rejects_wrong_dump_address(self):
        with self.assertRaises(ValueError):
            parse_words(PHY.replace("00123000","00124000"),ROOT)
if __name__=="__main__":
    unittest.main()
