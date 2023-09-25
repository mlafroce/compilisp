use llvm_sys::prelude::LLVMTypeRef;
use llvm_sys::target::LLVMTargetDataRef;

#[allow(unused)]
extern "C" {
    pub fn LLVMCompilispGetTypeAllocSize(D: LLVMTargetDataRef, Ty: LLVMTypeRef) -> u32;
}
