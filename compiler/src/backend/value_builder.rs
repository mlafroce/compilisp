use crate::backend::runtime::EMPTY_STR;
use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildGlobalStringPtr, LLVMBuildPointerCast, LLVMBuildStore, LLVMConstInt,
    LLVMInt1TypeInContext, LLVMInt32TypeInContext, LLVMInt8TypeInContext, LLVMPointerType,
};
use llvm_sys::prelude::{LLVMBool, LLVMBuilderRef, LLVMContextRef, LLVMValueRef};
use std::ffi::{c_ulonglong, CString};

pub struct ValueBuilder;

pub enum Value<'a> {
    GlobalString { name: &'a str, value: &'a str },
    ConstInt(i32),
    VarInt32(&'a str, Option<i32>),
    VarBool(&'a str, Option<bool>),
    //Integer(i32)
}

impl ValueBuilder {
    /// Builds value in stack
    /// # Safety
    /// Any LLVM function is unsafe. Context and builder must be valid.
    pub unsafe fn build_value(
        context: LLVMContextRef,
        builder: LLVMBuilderRef,
        value: &Value,
    ) -> LLVMValueRef {
        match value {
            Value::GlobalString { name, value } => {
                let c_value = CString::new(*value).unwrap();
                let c_name = CString::new(*name).unwrap();
                unsafe { LLVMBuildGlobalStringPtr(builder, c_value.as_ptr(), c_name.as_ptr()) }
            }
            Value::ConstInt(value) => {
                let bind_type_type = unsafe { LLVMInt8TypeInContext(context) };
                unsafe {
                    LLVMConstInt(bind_type_type, *value as c_ulonglong, LLVMBool::from(false))
                }
            }
            Value::VarInt32(name, init_value) => {
                let name = CString::new(*name).unwrap();
                let alloca_type = unsafe { LLVMInt32TypeInContext(context) };
                let alloca = unsafe { LLVMBuildAlloca(builder, alloca_type, name.as_ptr()) };
                if let Some(value) = *init_value {
                    // Create constant `num`
                    let const_value = unsafe {
                        LLVMConstInt(alloca_type, value as c_ulonglong, LLVMBool::from(false))
                    };
                    // Save constant in stack
                    unsafe { LLVMBuildStore(builder, const_value, alloca) };
                }
                alloca
            }
            Value::VarBool(name, init_value) => {
                let name = CString::new(*name).unwrap();
                let alloca_type = unsafe { LLVMInt1TypeInContext(context) };
                let alloca = unsafe { LLVMBuildAlloca(builder, alloca_type, name.as_ptr()) };
                if let Some(value) = *init_value {
                    let const_value = unsafe {
                        LLVMConstInt(alloca_type, value as c_ulonglong, LLVMBool::from(false))
                    };
                    // Save constant in stack
                    unsafe { LLVMBuildStore(builder, const_value, alloca) };
                }
                alloca
            }
        }
    }

    /// Cast any pointer type to i8*
    /// # Safety
    /// Any LLVM function is unsafe
    pub unsafe fn cast_opaque(
        context: LLVMContextRef,
        builder: LLVMBuilderRef,
        value_ref: &LLVMValueRef,
    ) -> LLVMValueRef {
        let cast_type = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
        // Cast stack address to *i8 (reuse previous i8 type)
        LLVMBuildPointerCast(builder, *value_ref, cast_type, EMPTY_STR.as_ptr())
    }
}
