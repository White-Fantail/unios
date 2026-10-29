import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2k_handler_host import expected_host_exit
class HandlerSmokeControl(unittest.TestCase):
    def test_preflight_not_clean_pass(self):self.assertEqual(expected_host_exit("preflight"),89)
    def test_generic_fatal_not_clean_pass(self):self.assertEqual(expected_host_exit("generic"),99)
    def test_df_direct_call_not_clean_pass(self):self.assertEqual(expected_host_exit("df"),101)
    def test_rejects_boot_pass(self):
        for mode in ("preflight","generic","df"):
            self.assertNotEqual(expected_host_exit(mode),65)
    def test_bad_mode_fail_closed(self):
        with self.assertRaises(ValueError):expected_host_exit("kernel")
if __name__=="__main__":unittest.main()
