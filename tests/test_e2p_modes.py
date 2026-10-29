import unittest,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2p_host import expected_host_exit
class E2pModes(unittest.TestCase):
    def test_final_map_only(self):self.assertEqual(expected_host_exit("preflight"),113)
    def test_ebs_terminal(self):self.assertEqual(expected_host_exit("exitbs"),115)
    def test_retains_real_fault_regressions(self):
        self.assertEqual(expected_host_exit("ud"),103)
        self.assertEqual(expected_host_exit("ist"),105)
        self.assertEqual(expected_host_exit("df"),109)
    def test_no_firmware_boot_success(self):
        for m in ("preflight","exitbs","ud","ist","df"):
            self.assertNotEqual(expected_host_exit(m),65)
    def test_bad_variant_fails(self):
        with self.assertRaises(ValueError):expected_host_exit("full-kernel")
if __name__=="__main__":unittest.main()
