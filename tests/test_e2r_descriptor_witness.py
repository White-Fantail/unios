import sys,struct,unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"scripts"))
from e2r_descriptor_witness import (
    verify_descriptor_page,DESCRIPTORS,MAIN_TOP,IST_TOP,GDT_CODE,GDT_DATA
)
NORMAL=0x14000200
DF=0x14000400

def fixture():
    data=bytearray(4096)
    def u64(o,v): struct.pack_into("<Q",data,o,v)
    def u16(o,v): struct.pack_into("<H",data,o,v)
    def u32(o,v): struct.pack_into("<I",data,o,v)
    u64(8,GDT_CODE)
    u64(16,GDT_DATA)
    base=DESCRIPTORS+0x100
    u64(24,103 | ((base & 0xffffff)<<16) | (0x89<<40) | (((base>>24)&0xff)<<56))
    u64(32,base>>32)
    u64(0x100+4,MAIN_TOP)
    u64(0x100+36,IST_TOP)
    u16(0x100+102,104)
    for vec in range(32):
        pos=0x200+16*vec
        handler=DF if vec==8 else NORMAL
        u16(pos,handler & 0xffff)
        u16(pos+2,8)
        data[pos+4]=1 if vec==8 else 0
        data[pos+5]=0x8e
        u16(pos+6,(handler>>16)&0xffff)
        u32(pos+8,handler>>32)
    return data

class ActualDescriptorHost(unittest.TestCase):
    def test_exact_good_dormant_page(self):
        p=verify_descriptor_page(fixture(),NORMAL,DF)
        self.assertTrue(p["descriptor_pa_contents_independently_verified"])
        self.assertFalse(p["descriptor_activated"])
        self.assertFalse(p["kernel_entry_executed"])
    def reject(self,mutator):
        b=fixture()
        mutator(b)
        with self.assertRaises(ValueError):verify_descriptor_page(b,NORMAL,DF)
    def test_truncated_page(self):self.reject(lambda b:b.pop())
    def test_bad_code_gdt(self):self.reject(lambda b:b.__setitem__(8,b[8]^1))
    def test_bad_tss_selector(self):self.reject(lambda b:b.__setitem__(29,b[29]^0x20))
    def test_bad_rsp0(self):self.reject(lambda b:b.__setitem__(0x104,b[0x104]^1))
    def test_bad_ist1(self):self.reject(lambda b:b.__setitem__(0x100+36,b[0x100+36]^1))
    def test_df_gate_no_ist(self):self.reject(lambda b:b.__setitem__(0x200+8*16+4,0))
    def test_normal_gate_bad(self):self.reject(lambda b:b.__setitem__(0x200+9*16,b[0x200+9*16]^1))
    def test_nonzero_reserved(self):self.reject(lambda b:b.__setitem__(4095,1))
    def test_untrusted_handler_pointer(self):
        with self.assertRaises(ValueError):verify_descriptor_page(fixture(),NORMAL,NORMAL)

if __name__=="__main__":unittest.main()
