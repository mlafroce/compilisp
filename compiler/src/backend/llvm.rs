use crate::ast::ModuleAst;
use crate::backend::compilisp_ir::CompilispIrGenerator;
use crate::backend::debuginfo_builder::DebugInfoBuilder;
use crate::backend::function_builder::FunctionBuilder;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::runtime::RuntimeCompiler;
use crate::backend::type_factory::TypeFactory;
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

    pub fn add_module(&self, root: ModuleAst) {
        unsafe {
            let target = LLVMGetDefaultTargetTriple();
            let builder = LLVMCreateBuilderInContext(self.context);
            let module_name = CString::new(root.source.as_str()).unwrap();

            let module = LLVMModuleCreateWithNameInContext(module_name.as_ptr(), self.context);
            LLVMSetTarget(module, target);
            let type_factory = TypeFactory::new(module);
            let function_factory = FunctionFactory::new_with_base(module, &type_factory);
            let di_builder = DebugInfoBuilder::new(module, &root.source);

            let main_block = self.build_main_function(module);
            LLVMPositionBuilderAtEnd(builder, main_block);

            let runtime = RuntimeCompiler::init(builder, function_factory, type_factory);

            let mut ir_generator = CompilispIrGenerator::new();
            let mut ir_buffer = vec![];
            for expr in root.expr_vec {
                ir_generator.process(expr);
                println!("IR: {:?}", ir_generator.ir_buffer);
                ir_buffer.append(&mut ir_generator.ir_buffer);
                ir_generator.ir_buffer = vec![];
            }
            runtime.process_ir(module, builder, ir_buffer);

            runtime.destroy(builder);

            //LLVMPositionBuilderAtEnd(builder, main_block);
            let ret_value = LLVMConstInt(
                LLVMInt32TypeInContext(self.context),
                0,
                LLVMBool::from(false),
            );
            LLVMBuildRet(builder, ret_value);

            let output_name = root.source.replace(".scheme", ".ll");
            let mut error_msg: *mut c_char = null_mut();
            println!("writing {output_name}");
            let output_name = CString::new(output_name).unwrap();
            LLVMPrintModuleToFile(module, output_name.as_ptr(), &mut error_msg);
            di_builder.finalize();
            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module)
        };
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

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
