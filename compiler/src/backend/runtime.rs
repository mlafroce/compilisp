use crate::backend::compilisp_ir::CompilispIr;
use crate::backend::compilisp_llvm_generator::CompilispLLVMGenerator;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::type_factory::TypeFactory;
use lazy_static::lazy_static;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::ffi::CString;
use std::ptr::null_mut;

/// Compiles scheme code using compilisp runtime calls
pub struct RuntimeCompiler {
    runtime_ref: LLVMValueRef,
    function_factory: FunctionFactory,
    type_factory: TypeFactory,
}

lazy_static! {
    pub static ref EMPTY_STR: CString = CString::new("").unwrap();
    pub static ref THEN_STR: CString = CString::new("then").unwrap();
    pub static ref ELSE_STR: CString = CString::new("else").unwrap();
    pub static ref FINALLY_STR: CString = CString::new("finally").unwrap();
}

impl RuntimeCompiler {
    pub unsafe fn init(
        builder: LLVMBuilderRef,
        function_factory: FunctionFactory,
        type_factory: TypeFactory,
    ) -> Self {
        let (fn_ref, fn_argtypes) = function_factory.get("compilisp_init").copied().unwrap();
        let runtime_ref = LLVMBuildCall2(
            builder,
            fn_argtypes,
            fn_ref,
            null_mut(),
            0,
            EMPTY_STR.as_ptr(),
        );
        Self {
            runtime_ref,
            function_factory,
            type_factory,
        }
    }

    pub unsafe fn destroy(self, builder: LLVMBuilderRef) {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_destroy")
            .copied()
            .unwrap();

        let mut args = [self.runtime_ref];
        LLVMBuildCall2(
            builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            1,
            EMPTY_STR.as_ptr(),
        );
    }

    pub unsafe fn process_ir<IRStream>(
        &self,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        ir_stream: IRStream,
    ) where
        IRStream: IntoIterator<Item = CompilispIr>,
    {
        let mut builder = CompilispLLVMGenerator::new(
            module,
            builder,
            self.runtime_ref,
            &self.function_factory,
            &self.type_factory,
        );
        for inst in ir_stream {
            builder.build_instruction(inst);
        }
    }
}
