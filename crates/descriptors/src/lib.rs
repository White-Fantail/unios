#![no_std]
// LAB-H1 E2f: construct and audit dormant x86_64 emergency descriptor
// tables. NO LGDT, LIDT, LTR or ExitBootServices is issued here.
// One 4KiB page contains GDT/TSS/32-vector IDT with disjoint ranges.
pub const PAGE: usize=4096;
pub const GDT_OFF: usize=0;
pub const GDT_BYTES: usize=40; // null, kernel code, data, 16-byte 64-bit TSS
pub const TSS_OFF: usize=0x100;
pub const TSS_BYTES: usize=104;
pub const IDT_OFF: usize=0x200;
pub const IDT_VECTORS: usize=32;
pub const IDT_BYTES: usize=IDT_VECTORS*16;
pub const KERNEL_CS: u16=0x08;
pub const KERNEL_DS: u16=0x10;
pub const TSS_SEL: u16=0x18;
pub const DOUBLE_FAULT: usize=8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Length, Alignment, NonCanonical, InvalidStack, InvalidHandler,
    InvalidTable, InvalidGdt, InvalidTss, InvalidIdt, Reserved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Image {
    pub gdtr_base:u64, pub gdtr_limit:u16,
    pub idtr_base:u64, pub idtr_limit:u16,
    pub tss_base:u64, pub tss_selector:u16,
}
fn canonical(v:u64)->bool {
    (v>>48)==if v&(1<<47)==0 {0} else {0xffff}
}
fn set16(b:&mut [u8],o:usize,v:u16){b[o..o+2].copy_from_slice(&v.to_le_bytes())}
fn set32(b:&mut [u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_le_bytes())}
fn set64(b:&mut [u8],o:usize,v:u64){b[o..o+8].copy_from_slice(&v.to_le_bytes())}
fn get16(b:&[u8],o:usize)->u16 {u16::from_le_bytes(b[o..o+2].try_into().unwrap())}
fn get32(b:&[u8],o:usize)->u32 {u32::from_le_bytes(b[o..o+4].try_into().unwrap())}
fn get64(b:&[u8],o:usize)->u64 {u64::from_le_bytes(b[o..o+8].try_into().unwrap())}
fn tss_descriptor(b:&mut [u8],o:usize,base:u64) {
    let limit=(TSS_BYTES-1) as u64;
    let low=(limit&0xffff)
       | ((base&0x00ff_ffff)<<16)
       | (0x89u64<<40) // P=1, DPL=0, available 64-bit TSS (type 9)
       | (((limit>>16)&0xf)<<48)
       | (((base>>24)&0xff)<<56);
    set64(b,o,low);
    set64(b,o+8,base>>32);
}
fn set_gate(b:&mut [u8],n:usize,handler:u64,ist:u8) {
    let o=IDT_OFF+n*16;
    set16(b,o,handler as u16);
    set16(b,o+2,KERNEL_CS);
    b[o+4]=ist;
    b[o+5]=0x8e; // P, ring0, 64-bit interrupt gate
    set16(b,o+6,(handler>>16) as u16);
    set32(b,o+8,(handler>>32) as u32);
    set32(b,o+12,0);
}
fn get_gate(b:&[u8],n:usize)->(u64,u16,u8,u8,u32) {
    let o=IDT_OFF+n*16;
    let dest=(get16(b,o) as u64)
        | ((get16(b,o+6) as u64)<<16)
        | ((get32(b,o+8) as u64)<<32);
    (dest,get16(b,o+2),b[o+4],b[o+5],get32(b,o+12))
}
pub fn build(
    b:&mut [u8], descriptors_va:u64, main_top:u64, df_ist_top:u64,
    normal_handler:u64, df_handler:u64,
)->Result<Image,Error> {
    if b.len()!=PAGE {return Err(Error::Length);}
    if descriptors_va & 4095!=0 {return Err(Error::Alignment);}
    if !canonical(descriptors_va) || !canonical(descriptors_va+0x300) {
        return Err(Error::NonCanonical);
    }
    if !canonical(main_top) || !canonical(df_ist_top) ||
       main_top & 15 !=0 || df_ist_top & 15 !=0 ||
       main_top==df_ist_top || main_top==0 || df_ist_top==0 {
        return Err(Error::InvalidStack);
    }
    if !canonical(normal_handler) || !canonical(df_handler) ||
        normal_handler < 0x10_0000 || df_handler < 0x10_0000 ||
        normal_handler >= (1<<32) || df_handler >= (1<<32) ||
        normal_handler==df_handler {
        return Err(Error::InvalidHandler);
    }
    b.fill(0);
    set64(b,8,0x00af9a000000ffff); // 64-bit ring0 code, L=1 D=0
    set64(b,16,0x00cf92000000ffff); // ring0 data
    let tss_va=descriptors_va+TSS_OFF as u64;
    tss_descriptor(b,24,tss_va);
    // x86_64 TSS offset 4: RSP0; offset 36: IST1; offset 102: IOPB.
    set64(b,TSS_OFF+4,main_top);
    set64(b,TSS_OFF+36,df_ist_top);
    set16(b,TSS_OFF+102,TSS_BYTES as u16);
    for v in 0..IDT_VECTORS {
        set_gate(b,v,if v==DOUBLE_FAULT {df_handler} else {normal_handler},
            if v==DOUBLE_FAULT {1} else {0});
    }
    let image=Image {
        gdtr_base:descriptors_va+GDT_OFF as u64,
        gdtr_limit:(GDT_BYTES-1) as u16,
        idtr_base:descriptors_va+IDT_OFF as u64,
        idtr_limit:(IDT_BYTES-1) as u16,
        tss_base:tss_va,tss_selector:TSS_SEL,
    };
    verify(b,image,main_top,df_ist_top,normal_handler,df_handler)?;
    Ok(image)
}
pub fn verify(
    b:&[u8],image:Image,main_top:u64,df_ist_top:u64,
    normal_handler:u64,df_handler:u64,
)->Result<(),Error> {
    verify_with_ud(b,image,main_top,df_ist_top,normal_handler,df_handler,None)
}

fn verify_with_ud(
    b:&[u8],image:Image,main_top:u64,df_ist_top:u64,
    normal_handler:u64,df_handler:u64,ud_handler:Option<u64>,
)->Result<(),Error> {
    if b.len()!=PAGE {return Err(Error::Length);}
    if image.gdtr_base & 4095 !=0 ||
       image.gdtr_limit as usize+1!=GDT_BYTES ||
       image.idtr_base!=image.gdtr_base+IDT_OFF as u64 ||
       image.idtr_limit as usize+1!=IDT_BYTES ||
       image.tss_base!=image.gdtr_base+TSS_OFF as u64 ||
       image.tss_selector!=TSS_SEL {return Err(Error::InvalidTable);}
    if get64(b,0)!=0 ||
       get64(b,8)!=0x00af9a000000ffff ||
       get64(b,16)!=0x00cf92000000ffff {
        return Err(Error::InvalidGdt);
    }
    let desc_base=((get64(b,24)>>16)&0x00ff_ffff)
        | (((get64(b,24)>>56)&0xff)<<24)
        | (get64(b,32)<<32);
    if ((get64(b,24)>>40)&0xff)!=0x89 ||
       get64(b,24)&0xffff != (TSS_BYTES-1) as u64 ||
       desc_base!=image.tss_base ||
       (get64(b,32)>>32)!=0 {
        return Err(Error::InvalidGdt);
    }
    if get64(b,TSS_OFF+4)!=main_top ||
       get64(b,TSS_OFF+36)!=df_ist_top ||
       get16(b,TSS_OFF+102)!=TSS_BYTES as u16 {
        return Err(Error::InvalidTss);
    }
    for i in 0..IDT_VECTORS {
        let (target,cs,ist,attr,zero)=get_gate(b,i);
        if target!=if i==DOUBLE_FAULT {df_handler}
                else if i==6 {ud_handler.unwrap_or(normal_handler)}
                else {normal_handler} ||
            cs!=KERNEL_CS || ist!=if i==DOUBLE_FAULT {1} else {0} ||
            attr!=0x8e || zero!=0 {
            return Err(Error::InvalidIdt);
        }
    }
    if b[GDT_BYTES..TSS_OFF].iter().any(|v|*v!=0) ||
       b[TSS_OFF+TSS_BYTES..IDT_OFF].iter().any(|v|*v!=0) ||
       b[IDT_OFF+IDT_BYTES..].iter().any(|v|*v!=0) {
        return Err(Error::Reserved);
    }
    Ok(())
}

/// E2l lab: one exception-specific vector 6 (#UD) gate; all other
/// gates preserve the reviewed E2f descriptor contract. The image is
/// built at an independently identity-verified PHYSICAL address, and
/// loaded only by a terminal disposable-QEMU test.
pub fn build_lab_ud(
    b:&mut [u8],descriptors_va:u64,main_top:u64,df_ist_top:u64,
    normal_handler:u64,df_handler:u64,ud_handler:u64,
)->Result<Image,Error> {
    if !canonical(ud_handler) || ud_handler<0x10_0000 ||
       ud_handler>=(1<<32) || ud_handler==normal_handler ||
       ud_handler==df_handler {return Err(Error::InvalidHandler);}
    let image=build(b,descriptors_va,main_top,df_ist_top,
                    normal_handler,df_handler)?;
    set_gate(b,6,ud_handler,0);
    verify_lab_ud(b,image,main_top,df_ist_top,
                  normal_handler,df_handler,ud_handler)?;
    Ok(image)
}
pub fn verify_lab_ud(
    b:&[u8],image:Image,main_top:u64,df_ist_top:u64,
    normal_handler:u64,df_handler:u64,ud_handler:u64,
)->Result<(),Error> {
    if !canonical(ud_handler) || ud_handler<0x10_0000 ||
       ud_handler>=(1<<32) || ud_handler==normal_handler ||
       ud_handler==df_handler {return Err(Error::InvalidHandler);}
    verify_with_ud(b,image,main_top,df_ist_top,
                   normal_handler,df_handler,Some(ud_handler))
}

/// E2m disposable QEMU-only double-fault injection: a real #GP must
/// occur while CPU is trying to deliver an earlier #GP. Make *only* the
/// vector 13 (#GP) gate's code selector invalid, beyond GDTR limit 39.
/// The valid vector 8 (#DF) IST1 gate remains untouched. NEVER use on
/// physical hardware or an operational IDT.
pub const LAB_BAD_GP_SELECTOR:u16=0x28;
pub fn build_lab_df(
    b:&mut [u8],descriptors_va:u64,main_top:u64,df_ist_top:u64,
    normal_handler:u64,df_handler:u64,ud_handler:u64,
)->Result<Image,Error> {
    let image=build_lab_ud(b,descriptors_va,main_top,df_ist_top,
                           normal_handler,df_handler,ud_handler)?;
    set16(b,IDT_OFF+13*16+2,LAB_BAD_GP_SELECTOR);
    verify_lab_df(b,image,main_top,df_ist_top,
                  normal_handler,df_handler,ud_handler)?;
    Ok(image)
}
pub fn verify_lab_df(
    b:&[u8],image:Image,main_top:u64,df_ist_top:u64,
    normal_handler:u64,df_handler:u64,ud_handler:u64,
)->Result<(),Error> {
    if b.len()!=PAGE{return Err(Error::Length);}
    if get16(b,IDT_OFF+13*16+2)!=LAB_BAD_GP_SELECTOR ||
       LAB_BAD_GP_SELECTOR<=image.gdtr_limit ||
       get_gate(b,DOUBLE_FAULT).2!=1 {
        return Err(Error::InvalidIdt);
    }
    // Clone the exact 4KiB page only inside the QEMU-only validation
    // path, repair the *single intended corrupted selector* and then
    // reuse every existing strict GDT/TSS/IDT/zero-fill rule.
    let mut expected=[0u8;PAGE];
    expected.copy_from_slice(b);
    set16(&mut expected,IDT_OFF+13*16+2,KERNEL_CS);
    verify_lab_ud(&expected,image,main_top,df_ist_top,
                  normal_handler,df_handler,ud_handler)
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    const VA:u64=0xffffffff83fc0000;
    const M:u64=0xffffffff83fe5000;
    const D:u64=0xffffffff83ff5000;
    const N:u64=0x1400_0200;
    const F:u64=0x1400_0400;
    fn fixture()->(std::vec::Vec<u8>,Image){
        let mut b=vec![0xaa;PAGE];
        let img=build(&mut b,VA,M,D,N,F).unwrap();
        (b,img)
    }
    #[test] fn bundle_roundtrip(){let (b,img)=fixture();assert_eq!(verify(&b,img,M,D,N,F),Ok(()));}
    #[test] fn rejects_non_page_sized_buffer(){
        let mut b=vec![0;PAGE-1];
        assert_eq!(build(&mut b,VA,M,D,N,F),Err(Error::Length));
    }
    #[test] fn rejects_unaligned_descriptor_base(){
        let mut b=vec![0;PAGE];
        assert_eq!(build(&mut b,VA+1,M,D,N,F),Err(Error::Alignment));
    }
    #[test] fn rejects_shared_stacks(){
        let mut b=vec![0;PAGE];
        assert_eq!(build(&mut b,VA,M,M,N,F),Err(Error::InvalidStack));
    }
    #[test] fn rejects_unaligned_stack(){
        let mut b=vec![0;PAGE];
        assert_eq!(build(&mut b,VA,M+8,D,N,F),Err(Error::InvalidStack));
    }
    #[test] fn rejects_same_handler(){
        let mut b=vec![0;PAGE];
        assert_eq!(build(&mut b,VA,M,D,N,N),Err(Error::InvalidHandler));
    }
    #[test] fn validates_double_fault_ist_only(){
        let (b,_) =fixture();
        for vector in 0..IDT_VECTORS {
            assert_eq!(b[IDT_OFF+vector*16+4],if vector==8 {1} else {0});
        }
    }
    #[test] fn rejects_corrupt_tss_rsp0(){
        let (mut b,img)=fixture(); b[TSS_OFF+4]^=1;
        assert_eq!(verify(&b,img,M,D,N,F),Err(Error::InvalidTss));
    }
    #[test] fn rejects_corrupt_df_ist(){
        let (mut b,img)=fixture();b[TSS_OFF+36]^=1;
        assert_eq!(verify(&b,img,M,D,N,F),Err(Error::InvalidTss));
    }
    #[test] fn rejects_wrong_idt_selector(){
        let (mut b,img)=fixture();b[IDT_OFF+3]^=4;
        assert_eq!(verify(&b,img,M,D,N,F),Err(Error::InvalidIdt));
    }
    #[test] fn rejects_nonpresent_df_gate(){
        let (mut b,img)=fixture();b[IDT_OFF+8*16+5]=0x0e;
        assert_eq!(verify(&b,img,M,D,N,F),Err(Error::InvalidIdt));
    }
    #[test] fn rejects_wrong_df_ist_gate(){
        let (mut b,img)=fixture();b[IDT_OFF+8*16+4]=0;
        assert_eq!(verify(&b,img,M,D,N,F),Err(Error::InvalidIdt));
    }
    #[test] fn rejects_nonzero_reserved(){
        let (mut b,img)=fixture();b[PAGE-1]=0x31;
        assert_eq!(verify(&b,img,M,D,N,F),Err(Error::Reserved));
    }
    #[test] fn live_lab_ud_gate_is_isolated_to_vector6(){
        let (mut bytes,_)=fixture();
        let image=build_lab_ud(&mut bytes,VA,M,D,N,F,0x1400_0600).unwrap();
        assert_eq!(verify_lab_ud(&bytes,image,M,D,N,F,0x1400_0600),Ok(()));
        assert_eq!(get_gate(&bytes,6).0,0x1400_0600);
        assert_eq!(get_gate(&bytes,8).0,F);
        assert_eq!(get_gate(&bytes,5).0,N);
        assert_eq!(verify(&bytes,image,M,D,N,F),Err(Error::InvalidIdt));
    }
    #[test] fn live_lab_rejects_missing_or_wrong_ud_gate(){
        let (mut bytes,_)=fixture();
        let image=build_lab_ud(&mut bytes,VA,M,D,N,F,0x1400_0600).unwrap();
        bytes[IDT_OFF+6*16]^=1;
        assert_eq!(verify_lab_ud(&bytes,image,M,D,N,F,0x1400_0600),Err(Error::InvalidIdt));
    }
    #[test] fn live_lab_rejects_alias_ud_handler(){
        let mut bytes=vec![0;PAGE];
        assert_eq!(build_lab_ud(&mut bytes,VA,M,D,N,F,N),Err(Error::InvalidHandler));
    }
    #[test] fn e2m_df_injection_only_gp_selector_is_broken(){
        let (mut b,_) = fixture();
        let df=0x1400_0800;
        let image=build_lab_df(&mut b,VA,M,D,N,df,0x1400_0600).unwrap();
        assert_eq!(get_gate(&b,13).1,LAB_BAD_GP_SELECTOR);
        assert_eq!(get_gate(&b,8).1,KERNEL_CS);
        assert_eq!(get_gate(&b,8).2,1);
        assert_eq!(get_gate(&b,6).0,0x1400_0600);
        assert_eq!(verify_lab_df(&b,image,M,D,N,df,0x1400_0600),Ok(()));
        assert_eq!(verify_lab_ud(&b,image,M,D,N,df,0x1400_0600),Err(Error::InvalidIdt));
    }
    #[test] fn e2m_df_injection_rejects_wrong_bad_selector(){
        let (mut b,_) = fixture();
        let df=0x1400_0800;
        let image=build_lab_df(&mut b,VA,M,D,N,df,0x1400_0600).unwrap();
        set16(&mut b,IDT_OFF+13*16+2,0x30);
        assert_eq!(verify_lab_df(&b,image,M,D,N,df,0x1400_0600),Err(Error::InvalidIdt));
    }
    #[test] fn e2m_df_injection_rejects_missing_ist(){
        let (mut b,_) = fixture();
        let df=0x1400_0800;
        let image=build_lab_df(&mut b,VA,M,D,N,df,0x1400_0600).unwrap();
        b[IDT_OFF+8*16+4]=0;
        assert_eq!(verify_lab_df(&b,image,M,D,N,df,0x1400_0600),Err(Error::InvalidIdt));
    }
    #[test] fn e2m_df_injection_rejects_corrupt_tss(){
        let (mut b,_) = fixture();
        let df=0x1400_0800;
        let image=build_lab_df(&mut b,VA,M,D,N,df,0x1400_0600).unwrap();
        b[TSS_OFF+36]^=1;
        assert_eq!(verify_lab_df(&b,image,M,D,N,df,0x1400_0600),Err(Error::InvalidTss));
    }
    #[test] fn rejects_corrupt_tss_descriptor(){
        let (mut b,img)=fixture();b[29]^=0x80;
        assert_eq!(verify(&b,img,M,D,N,F),Err(Error::InvalidGdt));
    }
}
