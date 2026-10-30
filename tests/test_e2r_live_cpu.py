import sys,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2r_live_tree import PACKET,COUNT,decode_manifest,cpu_stub_bounds
from e2r_live_cpu import verify_cpu
from e2o_new_tree import DESCRIPTORS

def targets():
    t=[{"va":0,"pa":0,"rights":0} for _ in range(COUNT)]
    t[25]={"va":0x14003000,"pa":0x14003000,"rights":1}
    t[26]={"va":0x14003100,"pa":0x14003100,"rights":1}
    return t

def regs(cr3=0xdf5d000,rip=0x14003040,rsp=0xe000000,gdtr=DESCRIPTORS,ss=16,tr=24):
    return (
        f"RSP={rsp:016x} RIP={rip:016x}\nCR3={cr3:016x}\n"
        f"GDT=     {gdtr:016x} 00000027\n"
        f"IDT=     {DESCRIPTORS+0x200:016x} 000001ff\n"
        f"CS ={8:04x} 0000000000000000 ffffffff 00af9a00\n"
        f"SS ={ss:04x} 0000000000000000 ffffffff 00cf9200\n"
        f"DS ={16:04x} 0000000000000000 ffffffff 00cf9200\n"
        f"ES ={16:04x} 0000000000000000 ffffffff 00cf9200\n"
        f"TR ={tr:04x} {DESCRIPTORS+0x100:016x} 00000067 00008b00\n"
    )

class TestActualDescriptorCPU(unittest.TestCase):
    def test_27_manifest(self):
        self.assertEqual(PACKET,10+27*17)
        with self.assertRaises(ValueError):decode_manifest(bytes(PACKET))
        with self.assertRaises(ValueError):decode_manifest(bytes(PACKET-1))
    def test_exact_cpu_stub(self):
        self.assertEqual(cpu_stub_bounds(targets()),(0x14003000,0x14003101))
    def test_bad_stub_writable(self):
        t=targets();t[25]["rights"]=2
        with self.assertRaises(ValueError):cpu_stub_bounds(t)
    def test_bad_stub_alias(self):
        t=targets();t[26]["pa"]+=4096
        with self.assertRaises(ValueError):cpu_stub_bounds(t)
    def test_actual_descriptor_cpu_regs(self):
        r=verify_cpu(regs(),0xdf5d000,0x14003000,0x14003101,0xdffe000,0xe000000)
        self.assertTrue(r["cpu_descriptor_activation_verified"])
        self.assertFalse(r["kernel_entry_executed"])
    def test_old_root(self):
        with self.assertRaises(ValueError):verify_cpu(regs(cr3=0xf801000),0xdf5d000,0x14003000,0x14003101,0xdffe000,0xe000000)
    def test_uefi_gdt(self):
        with self.assertRaises(ValueError):verify_cpu(regs(gdtr=0xf5dc000),0xdf5d000,0x14003000,0x14003101,0xdffe000,0xe000000)
    def test_uefi_ss(self):
        with self.assertRaises(ValueError):verify_cpu(regs(ss=0x30),0xdf5d000,0x14003000,0x14003101,0xdffe000,0xe000000)
    def test_null_tr(self):
        with self.assertRaises(ValueError):verify_cpu(regs(tr=0),0xdf5d000,0x14003000,0x14003101,0xdffe000,0xe000000)
    def test_unmapped_rip(self):
        with self.assertRaises(ValueError):verify_cpu(regs(rip=0x14005000),0xdf5d000,0x14003000,0x14003101,0xdffe000,0xe000000)

if __name__=="__main__":unittest.main()
