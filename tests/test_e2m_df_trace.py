import sys
import unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from audit_e2m_df_trace import verify_native_df_trace
SAMPLE=b"check_exception old: 0xd new 0xd\n  1: v=08 e=0000 i=0 cpl=0 IP=1234\n"
class NativeDfTrace(unittest.TestCase):
    def test_good_nested_gp_df(self):
        self.assertEqual(verify_native_df_trace(SAMPLE)["nested_gp_count"],1)
    def test_missing_nested_gp(self):
        with self.assertRaisesRegex(ValueError,"no independently"):
            verify_native_df_trace(b"1: v=08 e=0000 i=0\n")
    def test_software_interrupt_no_nested_gp(self):
        with self.assertRaisesRegex(ValueError,"no independently"):
            verify_native_df_trace(b"1: v=08 e=0000 i=1\n")
    def test_missing_df(self):
        with self.assertRaisesRegex(ValueError,"no zero"):
            verify_native_df_trace(b"check_exception old: 0xd new 0xd\n")
    def test_nonzero_error_code(self):
        with self.assertRaisesRegex(ValueError,"no zero"):
            verify_native_df_trace(b"check_exception old: 0xd new 0xd\n1: v=08 e=0001 i=0\n")
    def test_wrong_sequence(self):
        with self.assertRaisesRegex(ValueError,"no zero"):
            verify_native_df_trace(b"1: v=08 e=0000 i=0\ncheck_exception old: 0xd new 0xd\n")
    def test_triple_fault_refused(self):
        with self.assertRaisesRegex(ValueError,"triple"):
            verify_native_df_trace(SAMPLE+b"Triple fault\n")
    def test_empty_refused(self):
        with self.assertRaisesRegex(ValueError,"empty"):
            verify_native_df_trace(b"")
if __name__=="__main__":unittest.main()
