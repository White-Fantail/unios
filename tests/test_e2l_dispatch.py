import unittest
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2l_exception_host import expected_host_exit
class E2lDispatchCodes(unittest.TestCase):
    def test_preflight(self):self.assertEqual(expected_host_exit("preflight"),91)
    def test_ud(self):self.assertEqual(expected_host_exit("ud"),103)
    def test_ist(self):self.assertEqual(expected_host_exit("ist"),105)
    def test_fail_closed(self):
        for mode in ("preflight","ud","ist"):
            self.assertNotEqual(expected_host_exit(mode),65)
        with self.assertRaises(ValueError):expected_host_exit("df")
if __name__=="__main__":unittest.main()
