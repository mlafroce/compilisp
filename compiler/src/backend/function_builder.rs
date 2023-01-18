use llvm_sys::core::*;
use llvm_sys::prelude::{LLVMBool, LLVMModuleRef, LLVMTypeRef, LLVMValueRef};
use std::ffi::{c_uint, CString};

pub struct FunctionBuilder {
    name: CString,
    args: Vec<LLVMTypeRef>,
    ret_type: Option<LLVMTypeRef>,
}

impl FunctionBuilder {
    pub fn new() -> Self {
        let name = CString::new("").unwrap();
        Self {
            name,
            args: vec![],
            ret_type: None,
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = CString::new(name).unwrap();
        self
    }

    pub fn with_ret_type(mut self, ret_type: LLVMTypeRef) -> Self {
        self.ret_type = Some(ret_type);
        self
    }

    pub fn add_arg(mut self, arg_type: LLVMTypeRef) -> Self {
        self.args.push(arg_type);
        self
    }

    pub unsafe fn build(mut self, module: LLVMModuleRef) -> (LLVMValueRef, LLVMTypeRef) {
        let context = LLVMGetModuleContext(module);
        let ret_type = self.ret_type.unwrap_or(LLVMVoidTypeInContext(context));
        let args_size = self.args.len() as c_uint;
        let args_ptr = self.args.as_mut_ptr();
        let fn_type = LLVMFunctionType(ret_type, args_ptr, args_size, LLVMBool::from(false));
        let function = LLVMAddFunction(module, self.name.as_ptr(), fn_type);

        (function, fn_type)
    }
}
