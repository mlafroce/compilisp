use crate::backend::runtime::EMPTY_STR;
use llvm_sys::core::{LLVMBuildCondBr, LLVMBuildInBoundsGEP2, LLVMBuildLoad2, LLVMBuildRet, LLVMConstInt, LLVMInsertExistingBasicBlockAfterInsertBlock, LLVMInt32Type, LLVMPositionBuilderAtEnd};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBool, LLVMBuilderRef, LLVMTypeRef, LLVMValueRef};
use std::ffi::{c_uint, c_ulonglong};

pub struct Builder {
    builder: LLVMBuilderRef,
}

impl Builder {
    pub fn new(builder: LLVMBuilderRef) -> Self {
        Self { builder }
    }

    pub unsafe fn gep(
        &self,
        target: LLVMValueRef,
        target_type: LLVMTypeRef,
        indices: &[usize],
    ) -> LLVMValueRef {
        let mut idx_vec = vec![];
        for idx in indices {
            idx_vec.push(Builder::build_int(*idx));
        }
        LLVMBuildInBoundsGEP2(
            self.builder,
            target_type,
            target,
            idx_vec.as_mut_ptr(),
            idx_vec.len() as c_uint,
            EMPTY_STR.as_ptr(),
        )
    }

    pub unsafe fn load(&self, type_: LLVMTypeRef, value: LLVMValueRef) -> LLVMValueRef {
        LLVMBuildLoad2(self.builder, type_, value, EMPTY_STR.as_ptr())
    }

    pub unsafe fn cond_br(&self, cond_value: LLVMValueRef, block_then: LLVMBasicBlockRef, block_else: LLVMBasicBlockRef) -> LLVMValueRef {
        LLVMBuildCondBr(self.builder, cond_value, block_then, block_else)
    }

    pub unsafe fn ret(&self, value: LLVMValueRef) -> LLVMValueRef {
        LLVMBuildRet(self.builder, value)
    }

    pub unsafe fn insert_and_position_block(&self, block: LLVMBasicBlockRef) {
        LLVMInsertExistingBasicBlockAfterInsertBlock(self.builder, block);
        LLVMPositionBuilderAtEnd(self.builder, block);
    }

    fn build_int(n: usize) -> LLVMValueRef {
        unsafe { LLVMConstInt(LLVMInt32Type(), n as c_ulonglong, LLVMBool::from(false)) }
    }
}
