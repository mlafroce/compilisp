use crate::ast::Expr;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::runtime::EMPTY_STR;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::ffi::{c_uint, c_ulonglong, CString};
use crate::backend::expr_builder::ExprBuilder;

pub struct ProcedureCallBuilder<'a> {
    runtime_ref: LLVMValueRef,
    function_factory: &'a FunctionFactory,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    expr_builder: &'a ExprBuilder<'a>,
}

impl<'a> ProcedureCallBuilder<'a> {
    pub fn new(
        runtime_ref: LLVMValueRef,
        function_factory: &'a FunctionFactory,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        expr_builder: &'a ExprBuilder,
    ) -> Self {
        Self {
            runtime_ref,
            function_factory,
            module,
            builder,
            expr_builder
        }
    }
    /// Returns a tuple with result expr value ref
    pub unsafe fn process_procedure(
        &self,
        name: &str,
        args: &Vec<Expr>,
    ) -> (LLVMValueRef, LLVMValueRef) {
        for expr in args {
            self.procedure_generic_push_arg(expr);
        }
        self.procedure_generic_call(name, args.len())
    }

    unsafe fn procedure_generic_push_arg(&self, arg: &Expr) {
        let (bind_type, bind_value) = self.expr_builder.build_expr_in_stack(arg);
        self.procedure_push_arg_tuple(bind_type, bind_value);
    }

    unsafe fn procedure_push_arg_tuple(&self, arg_type: LLVMValueRef, arg_value: LLVMValueRef) {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_procedure_push_arg")
            .copied()
            .unwrap();

        let mut args = [self.runtime_ref, arg_type, arg_value];
        LLVMBuildCall2(
            self.builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            args.len() as c_uint,
            EMPTY_STR.as_ptr(),
        );
    }

    unsafe fn procedure_generic_call(&self, name: &str, stack_size: usize) -> (LLVMValueRef, LLVMValueRef) {
        let context = LLVMGetModuleContext(self.module);
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_procedure_call")
            .copied()
            .unwrap();
        let c_name = CString::new(name).unwrap();
        let c_name_var = CString::new("procedure_name").unwrap();
        let name_value =
            LLVMBuildGlobalStringPtr(self.builder, c_name.as_ptr(), c_name_var.as_ptr());

        let bind_type_type = LLVMInt8TypeInContext(context);
        let stack_size_value = LLVMConstInt(bind_type_type, stack_size as c_ulonglong , LLVMBool::from(false));

        let result_type_name = CString::new("res_type").unwrap();
        let result_name = CString::new("result").unwrap();
        let result_type_t = LLVMInt8TypeInContext(context);
        // Create stack space for result type (i8)
        let res_type_alloc =
            LLVMBuildAlloca(self.builder, result_type_t, result_type_name.as_ptr());
        // TODO: enable pointer results
        let result_t = LLVMInt32TypeInContext(context);
        // Create stack space for i32
        let result_alloc = LLVMBuildAlloca(self.builder, result_t, result_name.as_ptr());
        let opaque_ptr_t = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
        let result_alloc_i8 =
            LLVMBuildBitCast(self.builder, result_alloc, opaque_ptr_t, EMPTY_STR.as_ptr());

        let mut args = [
            self.runtime_ref,
            name_value,
            stack_size_value,
            res_type_alloc,
            result_alloc_i8,
        ];
        LLVMBuildCall2(
            self.builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            args.len() as c_uint,
            EMPTY_STR.as_ptr(),
        );
        (res_type_alloc, result_alloc_i8)
    }
}
