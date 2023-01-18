use crate::ast::Expr;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::procedure_call_builder::{build_expr_in_stack, ProcedureCallBuilder};
use lazy_static::lazy_static;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::ffi::{c_uint, CString};
use std::ptr::null_mut;

/// Compiles scheme code using compilisp runtime calls
pub struct RuntimeCompiler {
    runtime_ref: LLVMValueRef,
    function_factory: FunctionFactory,
}

lazy_static! {
    pub static ref EMPTY_STR: CString = CString::new("").unwrap();
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
        match expression {
            Expr::LetProcedure(bindings, expression) => {
                self.process_let(module, builder, bindings, expression.as_ref())
            }
            Expr::Procedure(name, args) => {
                let call_builder = ProcedureCallBuilder::new(
                    self.runtime_ref,
                    self.function_factory.clone(),
                    module,
                    builder,
                );
                call_builder.process_procedure(name, args)
            }
            _ => {
                unimplemented!("Cannot process this token yet {:?}", expression)
            }
        }
    }

    unsafe fn process_let(
        &self,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        bindings: &Vec<(String, Expr)>,
        expression: &Expr,
    ) -> (LLVMValueRef, LLVMValueRef) {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_push_let_context")
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

        for (binding_name, binding_expr) in bindings {
            self.bind_let_value(module, builder, &binding_name, binding_expr);
        }
        self.process_expr(module, builder, expression)
    }

    unsafe fn bind_let_value(
        &self,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        binding_name: &str,
        binding_expr: &Expr,
    ) {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_push_let_binding")
            .copied()
            .unwrap();
        let c_binding_name = CString::new(binding_name).unwrap();
        let name_value =
            LLVMBuildGlobalStringPtr(builder, c_binding_name.as_ptr(), EMPTY_STR.as_ptr());

        let (bind_type, bind_value) = build_expr_in_stack(
            self.runtime_ref,
            self.function_factory.clone(),
            module,
            builder,
            binding_expr,
        );
        let mut args = [self.runtime_ref, name_value, bind_type, bind_value];
        LLVMBuildCall2(
            builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            args.len() as c_uint,
            EMPTY_STR.as_ptr(),
        );
    }
}
