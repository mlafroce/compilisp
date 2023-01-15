use crate::ast::Expr;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::ffi::{c_char, CString};
use std::ptr::{null_mut};
use crate::backend::function::FunctionBuilder;
use crate::backend::runtime::{FunctionFactory, RuntimeCompiler};

pub struct Compiler {
    context: LLVMContextRef,
}

impl Compiler {
    pub fn new() -> Self {
        let context = unsafe { LLVMContextCreate() };
        Self { context }
    }

    pub fn add_module(&self, name: &str, root: Expr) {
        let module_name = CString::new(name).unwrap();
        let output_name = CString::new(name.replace(".scheme", ".ll")).unwrap();
        unsafe {
            let module = LLVMModuleCreateWithNameInContext(module_name.as_ptr(), self.context);
            let builder = LLVMCreateBuilderInContext(self.context);
            let function_factory = FunctionFactory::new_with_base(module);

            let main_block = self.build_main_function(module);
            LLVMPositionBuilderAtEnd(builder, main_block);

            let runtime = RuntimeCompiler::init(builder, function_factory);

            runtime.process_expr(module, builder,&root);

            runtime.destroy(builder);

            LLVMPositionBuilderAtEnd(builder, main_block);
            let ret_value = LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, LLVMBool::from(false));
            LLVMBuildRet(builder, ret_value);

            let mut error_msg: *mut c_char = null_mut();
            println!("writing {}", output_name.to_str().unwrap());
            LLVMPrintModuleToFile(module, output_name.as_ptr(), &mut error_msg);

            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module)
        };
    }

    unsafe fn build_main_function(&self, module: LLVMModuleRef) -> LLVMBasicBlockRef {
        let char_type= LLVMInt8TypeInContext(self.context);

        let builder = FunctionBuilder::new()
            .with_name("main")
            .with_ret_type(LLVMInt32TypeInContext(self.context))
            .add_arg(LLVMInt8TypeInContext(self.context))
            .add_arg(LLVMPointerType(LLVMPointerType(char_type, 0), 0));
        let main_function = builder.build(module);
        let entry_str = CString::new("entry").unwrap();
        LLVMAppendBasicBlockInContext(self.context, main_function.0, entry_str.as_ptr())
    }
}

impl Drop for Compiler {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(self.context) };
    }
}
