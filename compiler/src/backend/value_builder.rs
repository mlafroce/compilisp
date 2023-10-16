use crate::backend::llvm_builder::Builder;
use crate::backend::runtime::EMPTY_STR;
use crate::backend::type_factory::{CompilispType, TypeFactory};
use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildBitCast, LLVMBuildGlobalStringPtr, LLVMBuildStore, LLVMConstInt,
    LLVMInt1TypeInContext,
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
    VariableString { value: &'a str },
    ConstInt(i32),
    VarInt32(&'a str, Option<i32>),
    VarBool(&'a str, Option<bool>),
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
        type_factory: &TypeFactory,
    ) -> LLVMValueRef {
        let g_builder = Builder::new(builder);
        match value {
            Value::GlobalString { name, value } => {
                let escaped_value = value.replace("\\n", "\n");
                self.get_or_create_global_str(builder, &escaped_value, name)
            }
            Value::VariableString { value } => {
                let escaped_value = value.replace("\\n", "\n");

                let alloca_type = type_factory.get_type(CompilispType::CompilispObject);
                let alloca = unsafe { LLVMBuildAlloca(builder, alloca_type, EMPTY_STR.as_ptr()) };

                let type_attr_ptr = g_builder.gep(alloca, alloca_type, &[0, 0]);

                let const_disc_value = self.build_const_int(2, type_factory);
                unsafe { LLVMBuildStore(builder, const_disc_value, type_attr_ptr) };

                let global_str = self.get_or_create_global_str(builder, &escaped_value, "name");
                let value_attr_ptr = g_builder.gep(alloca, alloca_type, &[0, 1]);

                // Save constant in stack
                unsafe { LLVMBuildStore(builder, global_str, value_attr_ptr) };
                alloca
            }
            Value::ConstInt(value) => self.build_const_int(*value, type_factory),
            Value::VarInt32(name, init_value) => {
                let name = CString::new(*name).unwrap();
                let alloca_type = type_factory.get_type(CompilispType::CompilispObject);
                let alloca = unsafe { LLVMBuildAlloca(builder, alloca_type, name.as_ptr()) };
                if let Some(value) = *init_value {
                    // Create constant `num`
                    let type_attr_ptr = g_builder.gep(alloca, alloca_type, &[0, 0]);
                    let const_disc_value = self.build_const_int(0, type_factory);
                    unsafe { LLVMBuildStore(builder, const_disc_value, type_attr_ptr) };
                    let value_attr_ptr = g_builder.gep(alloca, alloca_type, &[0, 1]);
                    let int_type = type_factory.get_type(CompilispType::IntPtr);
                    let casted =
                        LLVMBuildBitCast(builder, value_attr_ptr, int_type, EMPTY_STR.as_ptr());
                    // Save constant in stack
                    let const_value = self.build_const_int(value, type_factory);
                    unsafe { LLVMBuildStore(builder, const_value, casted) };
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

    fn build_const_int(&self, value: i32, type_factory: &TypeFactory) -> LLVMValueRef {
        let bind_type_type = type_factory.get_type(CompilispType::Int);
        unsafe { LLVMConstInt(bind_type_type, value as c_ulonglong, LLVMBool::from(false)) }
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
