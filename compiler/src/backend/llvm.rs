use crate::ast::Expr;
use crate::backend::function_builder::FunctionBuilder;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::runtime::RuntimeCompiler;
use llvm_sys::core::*;
use llvm_sys::debuginfo::LLVMDWARFEmissionKind::LLVMDWARFEmissionKindFull;
use llvm_sys::debuginfo::{
    LLVMCreateDIBuilder, LLVMDIBuilderCreateCompileUnit, LLVMDIBuilderCreateFile,
    LLVMDIBuilderFinalize, LLVMDWARFSourceLanguage, LLVMDisposeDIBuilder,
};
use llvm_sys::prelude::*;
use llvm_sys::target_machine::LLVMGetDefaultTargetTriple;
use llvm_sys::LLVMModuleFlagBehavior;
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

            let output_name = name.replace(".scheme", ".ll");
            let di_builder = create_di_builder(module, name);

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

            LLVMDIBuilderFinalize(di_builder);

            let mut error_msg: *mut c_char = null_mut();
            println!("writing {output_name}");
            let output_name = CString::new(output_name).unwrap();
            LLVMPrintModuleToFile(module, output_name.as_ptr(), &mut error_msg);

            LLVMDisposeDIBuilder(di_builder);
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

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

unsafe fn create_di_builder(module: LLVMModuleRef, name: &str) -> LLVMDIBuilderRef {
    let debug_version = CString::new("Debug Info Version").unwrap();
    let debug_version_value =
        LLVMValueAsMetadata(LLVMConstInt(LLVMInt32Type(), 3, LLVMBool::from(false)));
    LLVMAddModuleFlag(
        module,
        LLVMModuleFlagBehavior::LLVMModuleFlagBehaviorWarning,
        debug_version.as_ptr(),
        debug_version.as_bytes().len(),
        debug_version_value,
    );
    let name_in_debuginfo = CString::new(name).unwrap();
    let work_dir = CString::new(".").unwrap();
    let di_producer = CString::new("LLVM Compilisp").unwrap();
    let split_name = CString::new("Unknown data").unwrap();
    let flags = "\0";

    let di_builder = LLVMCreateDIBuilder(module);
    let compile_unit_file = LLVMDIBuilderCreateFile(
        di_builder,
        name_in_debuginfo.as_ptr(),
        name_in_debuginfo.as_bytes().len(),
        work_dir.as_ptr().cast(),
        work_dir.as_bytes().len(),
    );
    LLVMDIBuilderCreateCompileUnit(
        di_builder,
        LLVMDWARFSourceLanguage::LLVMDWARFSourceLanguageC, // Should be Compilisp :P
        compile_unit_file,
        di_producer.as_ptr(),
        di_producer.as_bytes().len(),
        LLVMBool::from(false),
        flags.as_ptr().cast(),
        0,
        0,
        split_name.as_ptr(),
        split_name.as_bytes().len(),
        LLVMDWARFEmissionKindFull,
        1,
        LLVMBool::from(false),
        LLVMBool::from(false),
        "".as_ptr().cast(),
        0,
        "".as_ptr().cast(),
        0,
    );
    di_builder
}
