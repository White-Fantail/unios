import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2m_df_host import expected_host_exit
class RealDfHostModes(unittest.TestCase):
    def test_preflight(self):self.assertEqual(expected_host_exit("preflight"),93)
    def test_real_df(self):self.assertEqual(expected_host_exit("df"),109)
    def test_ud_regression(self):self.assertEqual(expected_host_exit("ud"),103)
    def test_ist_regression(self):self.assertEqual(expected_host_exit("ist"),105)
    def test_every_exit_distinct_from_clean_boot(self):
        for mode in ("preflight","df","ud","ist"):
            self.assertNotEqual(expected_host_exit(mode),65)
    def test_invalid_mode_rejected(self):
        with self.assertRaises(ValueError):expected_host_exit("no-source-proof")
if __name__=="__main__":unittest.main()
