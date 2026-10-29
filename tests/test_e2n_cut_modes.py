import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2n_cut_host import expected_host_exit
class E2nProofModes(unittest.TestCase):
    def test_uninstalled_cut_only(self):self.assertEqual(expected_host_exit("preflight"),95)
    def test_genuine_ud_regression(self):self.assertEqual(expected_host_exit("ud"),103)
    def test_int8_ist_regression(self):self.assertEqual(expected_host_exit("ist"),105)
    def test_genuine_df_regression(self):self.assertEqual(expected_host_exit("df"),109)
    def test_all_modes_are_not_kernel_boot(self):
        for mode in ("preflight","ud","ist","df"):
            self.assertNotEqual(expected_host_exit(mode),65)
    def test_reject_unknown(self):
        with self.assertRaises(ValueError):expected_host_exit("kernel")
if __name__=="__main__":unittest.main()
