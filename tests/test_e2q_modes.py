import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2q_host import expected_host_exit
class E2qModes(unittest.TestCase):
    def test_codes(self):
        self.assertEqual(expected_host_exit("preflight"),125)
        self.assertEqual(expected_host_exit("cr3"),121)
        self.assertEqual(expected_host_exit("exitbs"),115)
        self.assertEqual(expected_host_exit("ud"),103)
        self.assertEqual(expected_host_exit("ist"),105)
        self.assertEqual(expected_host_exit("df"),109)
    def test_not_clean_kernel(self):
        for mode in ("preflight","cr3","exitbs","ud","ist","df"):
            self.assertNotEqual(expected_host_exit(mode),65)
    def test_no_unknown(self):
        with self.assertRaises(ValueError):expected_host_exit("kernel")
if __name__=="__main__":unittest.main()
