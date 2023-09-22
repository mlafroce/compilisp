use llvm_sys::prelude::LLVMTypeRef;
use llvm_sys::target::LLVMTargetDataRef;

extern "C" {
    pub fn LLVMCompilispGetTypeAllocSize(D: LLVMTargetDataRef, Ty: LLVMTypeRef) -> u32;
}
