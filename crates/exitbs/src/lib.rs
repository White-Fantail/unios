#![no_std]
// L1 E2p: bounded EFI GetMemoryMap -> ExitBootServices transaction
// validation, with tests using *fake callbacks*. This crate itself
// never invokes real firmware or performs any CR3 changes.
pub const ATTEMPTS:usize=2;
pub const EFI_INVALID_PARAMETER:usize=(1usize<<(usize::BITS-1))|2;
pub const DESCRIPTOR_VERSION:u32=1;
pub const MAX_DESCRIPTOR_SIZE:usize=256;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Error { MapStatus,MapExtent,MapKey,MapStride,MapVersion,
                 ExitStatus,RetriesExhausted,PostSuccessCall }
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct Snapshot{pub bytes:usize,pub key:usize,pub stride:usize,pub version:u32}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum ExitDecision{Success,Retry}
pub fn validate_map(status:usize,s:Snapshot,capacity:usize)->Result<(),Error>{
    if status!=0 {return Err(Error::MapStatus);}
    if s.bytes==0 || s.bytes>capacity || capacity>64*1024 {
        return Err(Error::MapExtent);
    }
    if s.key==0 {return Err(Error::MapKey);}
    if s.stride<40 || s.stride%8!=0 || s.stride>MAX_DESCRIPTOR_SIZE ||
       s.bytes%s.stride!=0 {return Err(Error::MapStride);}
    if s.version!=DESCRIPTOR_VERSION {return Err(Error::MapVersion);}
    Ok(())
}
pub fn after_exit(status:usize,attempt:usize)->Result<ExitDecision,Error>{
    if attempt>=ATTEMPTS {return Err(Error::RetriesExhausted);}
    if status==0 {return Ok(ExitDecision::Success);}
    if status==EFI_INVALID_PARAMETER {
        if attempt+1<ATTEMPTS {return Ok(ExitDecision::Retry);}
        return Err(Error::RetriesExhausted);
    }
    Err(Error::ExitStatus)
}
// This deterministic model checks the exact zero-alloc call sequence
// and that a successful ExitBootServices causes no subsequent EFI calls.
// A real runner uses this same validate_map / after_exit policy.
pub trait Firmware {
    fn get_map(&mut self)->(usize,Snapshot);
    fn exit(&mut self,key:usize)->usize;
}
pub fn model<F:Firmware>(f:&mut F)->Result<usize,Error>{
    for attempt in 0..ATTEMPTS {
        let (status,s)=f.get_map();
        validate_map(status,s,64*1024)?;
        match after_exit(f.exit(s.key),attempt)? {
            ExitDecision::Success=>return Ok(attempt+1),
            ExitDecision::Retry=>{},
        }
    }
    Err(Error::RetriesExhausted)
}
#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;
    const GOOD:Snapshot=Snapshot{bytes:96,key:44,stride:48,version:1};
    struct Fake{statuses:[usize;2],reads:usize,exits:usize,calls:Vec<(u8,usize)>,key:usize}
    impl Fake {
        fn new(statuses:[usize;2])->Self{
            Self{statuses,reads:0,exits:0,calls:Vec::new(),key:44}
        }
    }
    impl Firmware for Fake{
        fn get_map(&mut self)->(usize,Snapshot){
            self.calls.push((1,self.reads));
            self.reads+=1;self.key+=1;
            (0,Snapshot{key:self.key,..GOOD})
        }
        fn exit(&mut self,key:usize)->usize{
            self.calls.push((2,key));
            let status=self.statuses[self.exits];self.exits+=1;
            status
        }
    }
    #[test]fn first_try_success_no_postcut_callbacks(){
        let mut f=Fake::new([0,0]);
        assert_eq!(model(&mut f),Ok(1));
        assert_eq!(f.calls,[(1,0),(2,45)]);
    }
    #[test]fn stale_key_retries_with_fresh_snapshot_only(){
        let mut f=Fake::new([EFI_INVALID_PARAMETER,0]);
        assert_eq!(model(&mut f),Ok(2));
        assert_eq!(f.calls,[(1,0),(2,45),(1,1),(2,46)]);
    }
    #[test]fn exhausted_retry_fails_closed(){
        let mut f=Fake::new([EFI_INVALID_PARAMETER,EFI_INVALID_PARAMETER]);
        assert_eq!(model(&mut f),Err(Error::RetriesExhausted));
        assert_eq!(f.reads,2);assert_eq!(f.exits,2);
    }
    #[test]fn other_exit_error_never_retries(){
        let mut f=Fake::new([7,0]);
        assert_eq!(model(&mut f),Err(Error::ExitStatus));
        assert_eq!(f.reads,1);assert_eq!(f.exits,1);
    }
    #[test]fn map_status_fails(){assert_eq!(validate_map(1,GOOD,65536),Err(Error::MapStatus));}
    #[test]fn zero_key_fails(){assert_eq!(validate_map(0,Snapshot{key:0,..GOOD},65536),Err(Error::MapKey));}
    #[test]fn bad_stride_fails(){
        assert_eq!(validate_map(0,Snapshot{stride:41,..GOOD},65536),Err(Error::MapStride));
    }
    #[test]fn truncated_map_fails(){
        assert_eq!(validate_map(0,Snapshot{bytes:97,..GOOD},65536),Err(Error::MapStride));
    }
    #[test]fn wrong_version_fails(){
        assert_eq!(validate_map(0,Snapshot{version:2,..GOOD},65536),Err(Error::MapVersion));
    }
    #[test]fn oversized_map_fails(){
        assert_eq!(validate_map(0,Snapshot{bytes:65537,..GOOD},65536),Err(Error::MapExtent));
    }
    #[test]fn unexpected_status_fails(){
        assert_eq!(after_exit(99,0),Err(Error::ExitStatus));
    }
    #[test]fn invalid_parameter_only_retry_allowed(){
        assert_eq!(after_exit(EFI_INVALID_PARAMETER,0),Ok(ExitDecision::Retry));
        assert_eq!(after_exit(EFI_INVALID_PARAMETER,1),Err(Error::RetriesExhausted));
    }
    #[test]fn no_third_attempt(){
        assert_eq!(after_exit(0,2),Err(Error::RetriesExhausted));
    }
}
