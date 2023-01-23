use crate::ast::Expr;
use crate::backend::function_factory::FunctionFactory;
use lazy_static::lazy_static;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::ffi::CString;
use std::ptr::null_mut;
use crate::backend::expr_builder::ExprBuilder;

/// Compiles scheme code using compilisp runtime calls
pub struct RuntimeCompiler {
    runtime_ref: LLVMValueRef,
    function_factory: FunctionFactory,
}

lazy_static! {
    pub static ref EMPTY_STR: CString = CString::new("").unwrap();
    pub static ref THEN_STR: CString = CString::new("then").unwrap();
    pub static ref ELSE_STR: CString = CString::new("else").unwrap();
    pub static ref FINALLY_STR: CString = CString::new("finally").unwrap();
}

impl RuntimeCompiler {
    pub unsafe fn init(builder: LLVMBuilderRef, function_factory: FunctionFactory) -> Self {
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

    pub unsafe fn process_expr(
        &self,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        expression: &Expr,
    ) -> (LLVMValueRef, LLVMValueRef) {
        let builder = ExprBuilder::new(module, builder,self.runtime_ref, &self.function_factory);
        builder.build_expr(expression)
    }

}
