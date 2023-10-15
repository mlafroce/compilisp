use crate::backend::runtime::EMPTY_STR;
use llvm_sys::core::{LLVMBuildInBoundsGEP2, LLVMConstInt, LLVMInt32Type};
use llvm_sys::prelude::{LLVMBool, LLVMBuilderRef, LLVMTypeRef, LLVMValueRef};
use std::ffi::{c_uint, c_ulonglong};

pub struct GepBuilder;

impl GepBuilder {
    pub unsafe fn build(
        builder: LLVMBuilderRef,
        target: LLVMValueRef,
        target_type: LLVMTypeRef,
        indices: &[usize],
    ) -> LLVMValueRef {
        let mut idx_vec = vec![];
        for idx in indices {
            idx_vec.push(GepBuilder::build_int(*idx));
        }
        LLVMBuildInBoundsGEP2(
            builder,
            target_type,
            target,
            idx_vec.as_mut_ptr(),
            idx_vec.len() as c_uint,
            EMPTY_STR.as_ptr(),
        )
    }

    unsafe fn build_int(n: usize) -> LLVMValueRef {
        LLVMConstInt(LLVMInt32Type(), n as c_ulonglong, LLVMBool::from(false))
    }
}
