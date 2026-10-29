import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2o_host import expected_host_exit
class E2oExitModes(unittest.TestCase):
    def test_private_pml4_witness_not_boot(self):self.assertEqual(expected_host_exit("preflight"),97)
    def test_faults_still_distinct(self):
        self.assertEqual(expected_host_exit("ud"),103)
        self.assertEqual(expected_host_exit("ist"),105)
        self.assertEqual(expected_host_exit("df"),109)
    def test_bad_variant_rejected(self):
        with self.assertRaises(ValueError):expected_host_exit("kernel")
if __name__=="__main__":unittest.main()
