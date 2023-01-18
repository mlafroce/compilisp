use crate::ast::Expr;
use crate::backend::function_builder::FunctionBuilder;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::runtime::RuntimeCompiler;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::target_machine::LLVMGetDefaultTargetTriple;
use std::ffi::{c_char, CString};
use std::ptr::null_mut;

pub struct Context {
    context: LLVMContextRef,
}

impl Context {
    pub fn new() -> Self {
        let context = unsafe { LLVMContextCreate() };
        Self { context }
    }

    pub fn add_module(&self, name: &str, root: Expr) {
        let module = self.create_module(name);
        unsafe {
            let target = LLVMGetDefaultTargetTriple();
            LLVMSetTarget(module, target);
            let builder = LLVMCreateBuilderInContext(self.context);
            let function_factory = FunctionFactory::new_with_base(module);

            let main_block = self.build_main_function(module);
            LLVMPositionBuilderAtEnd(builder, main_block);

            let runtime = RuntimeCompiler::init(builder, function_factory);

            runtime.process_expr(module, builder, &root);

            runtime.destroy(builder);

            LLVMPositionBuilderAtEnd(builder, main_block);
            let ret_value = LLVMConstInt(
                LLVMInt32TypeInContext(self.context),
                0,
                LLVMBool::from(false),
            );
            LLVMBuildRet(builder, ret_value);

            let mut error_msg: *mut c_char = null_mut();
            let output_name = name.replace(".scheme", ".ll");
            println!("writing {}", output_name);
            let output_name = CString::new(output_name).unwrap();
            LLVMPrintModuleToFile(module, output_name.as_ptr(), &mut error_msg);

            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module)
        };
    }

    fn create_module(&self, input_name: &str) -> LLVMModuleRef {
        let module_name = CString::new(input_name).unwrap();
        unsafe { LLVMModuleCreateWithNameInContext(module_name.as_ptr(), self.context) }
    }

    unsafe fn build_main_function(&self, module: LLVMModuleRef) -> LLVMBasicBlockRef {
        let char_type = LLVMInt8TypeInContext(self.context);

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

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(self.context) };
    }
}