import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2r_descriptor_new_tree import COUNT,PACKET,decode_manifest,descriptor_code_span
from e2r_new_tree import PACKET as PREVIOUS_PACKET

def fixture(begin=0x15001000,end=0x1500103f,rights=1):
    targets=[{"va":0,"pa":0,"rights":0} for _ in range(COUNT)]
    targets[23]={"va":begin,"pa":begin,"rights":rights}
    targets[24]={"va":end-1,"pa":end-1,"rights":1}
    return targets

class DescriptorCodePteWitness(unittest.TestCase):
    def test_count_and_packet(self):
        self.assertEqual(COUNT,25)
        self.assertEqual(PACKET-PREVIOUS_PACKET,34)
    def test_valid_64_byte_stub(self):
        self.assertEqual(descriptor_code_span(fixture()),(0x15001000,0x1500103f))
    def test_short_manifest(self):
        with self.assertRaises(ValueError):decode_manifest(bytes(PACKET-1))
    def test_wrong_manifest_count(self):
        with self.assertRaises(ValueError):decode_manifest(bytes(PACKET))
    def test_missing_last_code_target(self):
        with self.assertRaises(ValueError):descriptor_code_span(fixture()[:-1])
    def test_executable_writable_rejected(self):
        with self.assertRaises(ValueError):descriptor_code_span(fixture(rights=2))
    def test_not_identity_pa(self):
        a=fixture();a[24]["pa"]+=4096
        with self.assertRaises(ValueError):descriptor_code_span(a)
    def test_crosses_three_pages(self):
        with self.assertRaises(ValueError):
            descriptor_code_span(fixture(begin=0x15001ff0,end=0x15003010))
    def test_span_overlong(self):
        with self.assertRaises(ValueError):
            descriptor_code_span(fixture(begin=0x15001000,end=0x15001200))
    def test_high_half_fixture_rejected(self):
        with self.assertRaises(ValueError):
            descriptor_code_span(fixture(begin=0xffffffff80000000,end=0xffffffff8000003f))
    def test_reversed(self):
        with self.assertRaises(ValueError):
            descriptor_code_span(fixture(begin=0x15002000,end=0x15001000))

if __name__=="__main__":unittest.main()
