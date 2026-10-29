import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from audit_e2q_cr3_pe import audit,SWITCH
from audit_e2k_pe import Reject
from test_e2k_pe_audit import pe_fixture
class Cr3MachineCodeAudit(unittest.TestCase):
    def fixture(self):
        image=pe_fixture()
        image[0x280:0x280+len(SWITCH)]=SWITCH
        return image
    def test_linked_pe_machine_bytes(self):
        self.assertEqual(audit(self.fixture())[1],0x280)
    def test_missing_switch(self):
        d=self.fixture();d[0x280]^=1
        with self.assertRaises(Reject):audit(d)
    def test_duplicate_switch(self):
        d=self.fixture();d[0x2a0:0x2a0+len(SWITCH)]=SWITCH
        with self.assertRaises(Reject):audit(d)
    def test_writable_code_refused(self):
        d=self.fixture();d[0x188+39]|=0x80
        with self.assertRaises(Reject):audit(d)
    def test_wrong_order_refused(self):
        d=self.fixture();d[0x280:0x288]=bytes.fromhex("fa fc 0f 22 df 48 89 f4")
        with self.assertRaises(Reject):audit(d)
if __name__=="__main__":unittest.main()
