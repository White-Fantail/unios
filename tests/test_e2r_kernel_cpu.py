import unittest,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2r_kernel_cpu import verify_kernel,verify_context,MAIN_TOP,MAGIC,BASE
from e2o_new_tree import MAIN_STACK,CONTEXT,DESCRIPTORS
from e2r_live_cpu import field

ROOT=0xdf5d000
PA=0xde01000
def regs(rip=BASE+0x120,rsp=MAIN_TOP-0x58,cr3=ROOT,cs=8,gdt=DESCRIPTORS):
    return (
        f"RIP={rip:016x} RSP={rsp:016x} CR3={cr3:016x}\n"
        f"GDT=     {gdt:016x} 00000027\n"
        f"IDT=     {DESCRIPTORS+0x200:016x} 000001ff\n"
        f"CS ={cs:04x} 0000000000000000 ffffffff 00af9a00\n"
        f"SS =0010 0000000000000000 ffffffff 00cf9200\n"
        f"DS =0010 0000000000000000 ffffffff 00cf9200\n"
        f"ES =0010 0000000000000000 ffffffff 00cf9200\n"
        f"TR =0018 {DESCRIPTORS+0x100:016x} 00000067 00008b00\n")
def mapped(va):
    if BASE<=va<BASE+4096:return PA+va-BASE,1
    if MAIN_STACK<=va<MAIN_TOP:return 0xdead000+va-MAIN_STACK,2
    return None

class KernelHandoffOracle(unittest.TestCase):
    def test_good_cpu(self):
        p=verify_kernel(regs(),ROOT,BASE,PA,mapped)
        self.assertTrue(p["kernel_cpu_active_descriptors"])
        self.assertTrue(p["kernel_cpu_rx_physical_entry_provenance"])
    def test_old_cr3(self):
        with self.assertRaises(ValueError):verify_kernel(regs(cr3=0xf801000),ROOT,BASE,PA,mapped)
    def test_low_rip(self):
        with self.assertRaises(ValueError):verify_kernel(regs(rip=0xdfe3555),ROOT,BASE,PA,mapped)
    def test_low_rsp(self):
        with self.assertRaises(ValueError):verify_kernel(regs(rsp=0xdfe0000),ROOT,BASE,PA,mapped)
    def test_wrong_cs(self):
        with self.assertRaises(ValueError):verify_kernel(regs(cs=0x30),ROOT,BASE,PA,mapped)
    def test_wrong_gdt(self):
        with self.assertRaises(ValueError):verify_kernel(regs(gdt=0xdfe3000),ROOT,BASE,PA,mapped)
    def test_wrong_rx_phys(self):
        with self.assertRaises(ValueError):verify_kernel(regs(),ROOT,BASE,PA+4096,mapped)
    def test_guarded_stack(self):
        with self.assertRaises(ValueError):verify_kernel(regs(),ROOT,BASE,PA,lambda va:None if va>=MAIN_STACK else mapped(va))
    def test_context(self):
        words=[MAGIC,1,BASE,MAIN_TOP,ROOT,CONTEXT,0,0]
        p=verify_context(words,ROOT,BASE)
        self.assertTrue(p["boot_context_pa_bytes_independently_verified"])
    def test_bad_context_entry(self):
        words=[MAGIC,1,BASE+8,MAIN_TOP,ROOT,CONTEXT,0,0]
        with self.assertRaises(ValueError):verify_context(words,ROOT,BASE)
    def test_bad_context_magic(self):
        words=[0,1,BASE,MAIN_TOP,ROOT,CONTEXT,0,0]
        with self.assertRaises(ValueError):verify_context(words,ROOT,BASE)
    def test_stale_context_tail(self):
        words=[MAGIC,1,BASE,MAIN_TOP,ROOT,CONTEXT,1,0]
        with self.assertRaises(ValueError):verify_context(words,ROOT,BASE)
if __name__=="__main__":unittest.main()
