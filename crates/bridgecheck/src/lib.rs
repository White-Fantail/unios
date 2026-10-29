#![no_std]

// Metadata-only E2e bounded EFI code-span proof. It does NOT perform a
// CR3 switch or prove future RIP/stack/descriptor state at ExitBS.
pub const PAGE: u64 = 4096;
pub const MAX_CODE_BYTES: u64 = 1024;
pub const MAX_CODE_PAGES: usize = 2;
const EFI_LOW_MIN: u64 = 0x10_0000;
const EFI_LOW_END: u64 = 1 << 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Empty, Oversize, PageCount, NonCanonical, Address, Count,
    PageOrder, ChangedPhysical, Executable, User, Aliased, Ownership,
    CpuProtection, StubBytes,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub begin: u64,
    pub end: u64,
    pub first_page: u64,
    pub pages: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page {
    pub linear: u64,
    pub old_physical: u64,
    pub new_physical: u64,
    pub old_executable: bool,
    pub new_executable: bool,
    pub old_supervisor: bool,
    pub new_supervisor: bool,
    pub efi_loader_code: bool,
}

pub const EMERGENCY_BYTES: usize=11;
/// Exact 64-bit fail-stop handler: CLI, MOV DX,0xf4, MOV AL,code,
/// OUT DX,AL, HLT, short JMP back to HLT. This is NOT IRETQ.
/// Bytes are validated against *linked* UEFI code in E2k.
pub fn validate_emergency_handler(bytes: &[u8], signal: u8)
    -> Result<(),Error>
{
    // Restrict to previously reserved fatal/non-PASS debug-exit bytes.
    if signal != 0x31 && signal != 0x32 {return Err(Error::StubBytes);}
    if bytes.len()!=EMERGENCY_BYTES {return Err(Error::StubBytes);}
    let expected=[0xfa,0x66,0xba,0xf4,0x00,0xb0,signal,
                  0xee,0xf4,0xeb,0xfd];
    if bytes != expected {return Err(Error::StubBytes);}
    Ok(())
}

pub fn validate_preflight_stub(bytes: &[u8]) -> Result<(),Error> {
    // Exactly cli; cld; nop; ret. Never executed during E2e.
    if bytes != [0xfa,0xfc,0x90,0xc3] {return Err(Error::StubBytes);}
    Ok(())
}

impl Span {
    pub fn new(begin: u64, end: u64) -> Result<Self, Error> {
        if end <= begin { return Err(Error::Empty); }
        let len = end.checked_sub(begin).ok_or(Error::Address)?;
        if len > MAX_CODE_BYTES { return Err(Error::Oversize); }
        if begin < EFI_LOW_MIN || end > EFI_LOW_END { return Err(Error::Address); }
        let first_page = begin & !(PAGE - 1);
        let last = (end - 1) & !(PAGE - 1);
        let pages = ((last - first_page) / PAGE + 1) as usize;
        if pages > MAX_CODE_PAGES { return Err(Error::PageCount); }
        Ok(Self { begin, end, first_page, pages })
    }
    pub fn page(&self, n: usize) -> Option<u64> {
        if n >= self.pages { None } else { Some(self.first_page + n as u64 * PAGE) }
    }
}

pub fn verify(span: Span, evidence: &[Page], wp: bool, nxe: bool)
    -> Result<(), Error>
{
    if evidence.len() != span.pages { return Err(Error::Count); }
    if !wp || !nxe { return Err(Error::CpuProtection); }
    for (i, page) in evidence.iter().enumerate() {
        if page.linear != span.page(i).ok_or(Error::Count)? {
            return Err(Error::PageOrder);
        }
        if page.old_physical == 0 ||
            page.old_physical & (PAGE-1) != 0 ||
            page.new_physical & (PAGE-1) != 0 {
            return Err(Error::Address);
        }
        if page.old_physical != page.new_physical {
            return Err(Error::ChangedPhysical);
        }
        if !page.old_executable || !page.new_executable {
            return Err(Error::Executable);
        }
        if !page.old_supervisor || !page.new_supervisor {
            return Err(Error::User);
        }
        if !page.efi_loader_code { return Err(Error::Ownership); }
        for earlier in &evidence[..i] {
            if earlier.old_physical == page.old_physical {
                return Err(Error::Aliased);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> (Span, [Page; 2]) {
        let span = Span::new(0x10fffe,0x110008).unwrap();
        let data = [
            Page{linear:0x10f000,old_physical:0x8f000,
                 new_physical:0x8f000,old_executable:true,
                 new_executable:true,old_supervisor:true,
                 new_supervisor:true,efi_loader_code:true},
            Page{linear:0x110000,old_physical:0x90000,
                 new_physical:0x90000,old_executable:true,
                 new_executable:true,old_supervisor:true,
                 new_supervisor:true,efi_loader_code:true}
        ];
        (span,data)
    }
    #[test] fn checked_instruction_bytes() {
        assert_eq!(validate_preflight_stub(&[0xfa,0xfc,0x90,0xc3]),Ok(()));
        assert_eq!(validate_preflight_stub(&[0xfa,0xfc,0x90,0x90]),Err(Error::StubBytes));
        assert_eq!(validate_preflight_stub(&[0xfa,0xfc,0x90]),Err(Error::StubBytes));
    }
    #[test] fn emergency_generic_exact_bytes() {
        assert_eq!(validate_emergency_handler(&[
            0xfa,0x66,0xba,0xf4,0x00,0xb0,0x31,0xee,0xf4,0xeb,0xfd],0x31),Ok(()));
    }
    #[test] fn emergency_df_exact_bytes() {
        assert_eq!(validate_emergency_handler(&[
            0xfa,0x66,0xba,0xf4,0x00,0xb0,0x32,0xee,0xf4,0xeb,0xfd],0x32),Ok(()));
    }
    #[test] fn emergency_byte_mutation_rejected() {
        let mut b=[0xfa,0x66,0xba,0xf4,0x00,0xb0,0x31,0xee,0xf4,0xeb,0xfd];
        b[8]=0x90;
        assert_eq!(validate_emergency_handler(&b,0x31),Err(Error::StubBytes));
    }
    #[test] fn emergency_wrong_exit_byte_rejected() {
        let b=[0xfa,0x66,0xba,0xf4,0x00,0xb0,0x31,0xee,0xf4,0xeb,0xfd];
        assert_eq!(validate_emergency_handler(&b,0x32),Err(Error::StubBytes));
    }
    #[test] fn emergency_clean_exit_forbidden() {
        let b=[0xfa,0x66,0xba,0xf4,0x00,0xb0,0x20,0xee,0xf4,0xeb,0xfd];
        assert_eq!(validate_emergency_handler(&b,0x20),Err(Error::StubBytes));
    }
    #[test] fn emergency_truncation_rejected() {
        let b=[0xfa,0x66,0xba,0xf4,0x00,0xb0,0x31,0xee,0xf4,0xeb,0xfd];
        assert_eq!(validate_emergency_handler(&b[..10],0x31),Err(Error::StubBytes));
    }
    #[test] fn valid_two_page_span(){let (s,p)=sample();assert_eq!(verify(s,&p,true,true),Ok(()));}
    #[test] fn empty_span_rejected(){assert_eq!(Span::new(0x110000,0x110000),Err(Error::Empty));}
    #[test] fn reversed_span_rejected(){assert_eq!(Span::new(0x110001,0x110000),Err(Error::Empty));}
    #[test] fn oversized_span_rejected(){assert_eq!(Span::new(0x110000,0x110401),Err(Error::Oversize));}
    #[test] fn low_address_rejected(){assert_eq!(Span::new(0xfffff,0x100001),Err(Error::Address));}
    #[test] fn upper_address_rejected(){assert_eq!(Span::new(0xffff_ffff,0x1_0000_0001),Err(Error::Address));}
    #[test] fn count_mismatch_rejected(){let (s,p)=sample();assert_eq!(verify(s,&p[..1],true,true),Err(Error::Count));}
    #[test] fn page_order_rejected(){let (s,mut p)=sample();p[1].linear+=PAGE;assert_eq!(verify(s,&p,true,true),Err(Error::PageOrder));}
    #[test] fn changed_physical_rejected(){let (s,mut p)=sample();p[0].new_physical+=PAGE;assert_eq!(verify(s,&p,true,true),Err(Error::ChangedPhysical));}
    #[test] fn missing_execute_rejected(){let (s,mut p)=sample();p[1].old_executable=false;assert_eq!(verify(s,&p,true,true),Err(Error::Executable));}
    #[test] fn user_page_rejected(){let (s,mut p)=sample();p[0].new_supervisor=false;assert_eq!(verify(s,&p,true,true),Err(Error::User));}
    #[test] fn unowned_code_rejected(){let (s,mut p)=sample();p[0].efi_loader_code=false;assert_eq!(verify(s,&p,true,true),Err(Error::Ownership));}
    #[test] fn alias_rejected(){let (s,mut p)=sample();p[1].old_physical=p[0].old_physical;p[1].new_physical=p[0].new_physical;assert_eq!(verify(s,&p,true,true),Err(Error::Aliased));}
    #[test] fn missing_wp_rejected(){let (s,p)=sample();assert_eq!(verify(s,&p,false,true),Err(Error::CpuProtection));}
    #[test] fn missing_nxe_rejected(){let (s,p)=sample();assert_eq!(verify(s,&p,true,false),Err(Error::CpuProtection));}
}
