import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from l1_oracle import Observation, STAGES, clean_pass

class OracleTest(unittest.TestCase):
    def good(self, **changes):
        data = dict(case_id="L1-X01", expected_case_id="L1-X01",
                    serial="\n".join(STAGES) + "\n", exit_code=65,
                    timed_out=False, qmp_events=[])
        data.update(changes)
        return Observation(**data)

    def test_simulated_positive(self):
        self.assertTrue(clean_pass(self.good()))

    def test_uefi_preflight_cannot_be_clean_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=67)))

    def test_e2a_preflight_cannot_be_clean_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=69)))

    def test_e2b_staging_cannot_be_clean_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=71)))

    def test_e2c_private_tables_not_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=73)))

    def test_e2d_anchor_cannot_be_clean_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=75)))

    def test_e2e_span_is_not_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=77)))

    def test_e2f_dormant_descriptors_not_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=79)))

    def test_e2g_root_preflight_not_kernel_pass(self):
        self.assertFalse(clean_pass(self.good(exit_code=81)))

    def test_marker_then_reset(self):
        self.assertFalse(clean_pass(self.good(qmp_events=["RESET"])))

    def test_marker_then_hang(self):
        self.assertFalse(clean_pass(self.good(exit_code=None, timed_out=True)))

    def test_debug_exit_without_serial(self):
        self.assertFalse(clean_pass(self.good(serial="")))

    def test_lookalike_negative(self):
        self.assertFalse(clean_pass(self.good(case_id="L1-X09",
                                              expected_case_id="L1-X09")))

    def test_early_marker(self):
        self.assertFalse(clean_pass(self.good(serial="\n".join(reversed(STAGES)))))

    def test_duplicate_marker(self):
        self.assertFalse(clean_pass(self.good(serial="\n".join(STAGES + (STAGES[-1],)))))

    def test_missing_stage(self):
        self.assertFalse(clean_pass(self.good(serial="\n".join(STAGES[1:]))))

if __name__ == "__main__":
    unittest.main()
