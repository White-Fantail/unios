import unittest,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2r_clean_oracle import verify_trace,ENTRY,SUCCESS
class CleanBootOracle(unittest.TestCase):
    def test_full_proof(self):
        self.assertTrue(verify_trace(ENTRY+"\r\n"+SUCCESS+"\r\n",65,True)["full_clean_boot"])
    def reject(self,trace,code=65,qmp=True):
        with self.assertRaises(ValueError):verify_trace(trace,code,qmp)
    def test_no_qmp(self):self.reject(ENTRY+SUCCESS,65,False)
    def test_wrong_diagnostic_exit(self):self.reject(ENTRY+SUCCESS,133,True)
    def test_premature_success(self):self.reject(SUCCESS+ENTRY)
    def test_marker_alone(self):self.reject(SUCCESS)
    def test_entry_alone(self):self.reject(ENTRY)
    def test_duplicate_success(self):self.reject(ENTRY+SUCCESS+SUCCESS)
    def test_duplicate_entry(self):self.reject(ENTRY+ENTRY+SUCCESS)
    def test_rejected_boot(self):self.reject(ENTRY+"UNIOS:L1:E1_REJECTED"+SUCCESS)
    def test_unverified_kernel(self):self.reject(ENTRY+"UNIOS:L1:KERNEL_ENTRY_UNVERIFIED"+SUCCESS)
if __name__=="__main__":unittest.main()
