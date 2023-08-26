use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildGlobalStringPtr, LLVMBuildStore, LLVMConstInt, LLVMInt1TypeInContext,
    LLVMInt32TypeInContext, LLVMInt8TypeInContext,
};
use llvm_sys::prelude::{LLVMBool, LLVMBuilderRef, LLVMContextRef, LLVMValueRef};
use std::collections::HashMap;
use std::ffi::{c_ulonglong, CString};

#[derive(Default)]
pub struct ValueBuilder {
    global_strings: HashMap<String, LLVMValueRef>,
}
#[derive(Debug)]
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
        &mut self,
        context: LLVMContextRef,
        builder: LLVMBuilderRef,
        value: &Value,
    ) -> LLVMValueRef {
        match value {
            Value::GlobalString { name, value } => {
                let escaped_value = value.replace("\\n", "\n");
                self.get_or_create_global_str(builder, &escaped_value, name)
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

    pub fn get_or_create_global_str(
        &mut self,
        builder: LLVMBuilderRef,
        value: &str,
        name: &str,
    ) -> LLVMValueRef {
        if let Some(value_ref) = self.global_strings.get(value) {
            *value_ref
        } else {
            let c_value = CString::new(value).unwrap();
            let c_name = CString::new(name).unwrap();
            let value_ref =
                unsafe { LLVMBuildGlobalStringPtr(builder, c_value.as_ptr(), c_name.as_ptr()) };
            self.global_strings.insert(value.to_owned(), value_ref);
            value_ref
        }
    }
}
