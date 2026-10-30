import unittest,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2r_new_tree import COUNT,PACKET,decode_manifest,require_high_span
from e2q_new_tree import PACKET as OLD_PACKET

def sample(start=0x14001000,end=0x1400101a,rights=1):
    v=[{"va":0,"pa":0,"rights":0} for _ in range(COUNT)]
    v[21]={"va":start,"pa":start,"rights":rights}
    v[22]={"va":end-1,"pa":end-1,"rights":1}
    return v

class HighEntryPrecut(unittest.TestCase):
    def test_packet_is_explicitly_larger_than_e2q(self):
        self.assertEqual(PACKET,10+23*17)
        self.assertEqual(PACKET-OLD_PACKET,34)
    def test_wrong_manifest_count_and_truncation_rejected(self):
        with self.assertRaises(ValueError):decode_manifest(b"\0"*PACKET)
        with self.assertRaises(ValueError):decode_manifest(b"\0"*(PACKET-1))
    def test_valid_finite_low_PE_span(self):
        self.assertEqual(require_high_span(sample()),(0x14001000,0x1400101a))
    def test_untrusted_pa(self):
        v=sample();v[21]["pa"]^=4096
        with self.assertRaises(ValueError):require_high_span(v)
    def test_wrong_rights(self):
        with self.assertRaises(ValueError):require_high_span(sample(rights=2))
    def test_noncanonical_high_entry_stub(self):
        with self.assertRaises(ValueError):require_high_span(sample(start=0xffffffff80000000,end=0xffffffff8000001a))
    def test_crosses_more_than_two_pages(self):
        with self.assertRaises(ValueError):require_high_span(sample(start=0x14001ff0,end=0x14002040))
    def test_oversized(self):
        with self.assertRaises(ValueError):require_high_span(sample(start=0x14001000,end=0x14001042))
    def test_wrong_order(self):
        with self.assertRaises(ValueError):require_high_span(sample(start=0x14002000,end=0x14001000))
    def test_count(self):
        with self.assertRaises(ValueError):require_high_span(sample()[:-1])

if __name__=="__main__":unittest.main()
