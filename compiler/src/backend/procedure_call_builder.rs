use crate::ast::Expr;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::runtime::EMPTY_STR;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::ffi::{c_uint, c_ulonglong, CString};

pub struct ProcedureCallBuilder {
    runtime_ref: LLVMValueRef,
    function_factory: FunctionFactory,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
}

impl ProcedureCallBuilder {
    pub fn new(
        runtime_ref: LLVMValueRef,
        function_factory: FunctionFactory,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
    ) -> Self {
        Self {
            runtime_ref,
            function_factory,
            module,
            builder,
        }
    }
    /// Returns a tuple with result expr value ref
    pub unsafe fn process_procedure(
        &self,
        name: &str,
        args: &Vec<Expr>,
    ) -> (LLVMValueRef, LLVMValueRef) {
        for expr in args {
            self.procedure_push_arg(expr);
        }
        self.procedure_call(name)
    }

    unsafe fn procedure_push_arg(&self, arg: &Expr) {
        let (bind_type, bind_value) = build_expr_in_stack(
            self.runtime_ref,
            self.function_factory.clone(),
            self.module,
            self.builder,
            arg,
        );
        self.procedure_push_arg_tuple(bind_type, bind_value);
    }

    unsafe fn procedure_push_arg_tuple(&self, arg_type: LLVMValueRef, arg_value: LLVMValueRef) {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_procedure_push_arg")
            .copied()
            .unwrap();

        let mut args = [self.runtime_ref.clone(), arg_type, arg_value];
        LLVMBuildCall2(
            self.builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            args.len() as c_uint,
            EMPTY_STR.as_ptr(),
        );
    }

    unsafe fn procedure_call(&self, name: &str) -> (LLVMValueRef, LLVMValueRef) {
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
            self.runtime_ref.clone(),
            name_value,
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
        return (res_type_alloc, result_alloc_i8);
    }
}

/// Returns a tuple with (expr_type, expr_value) pointers
pub unsafe fn build_expr_in_stack(
    runtime_ref: LLVMValueRef,
    function_factory: FunctionFactory,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    expr: &Expr,
) -> (LLVMValueRef, LLVMValueRef) {
    let context = LLVMGetModuleContext(module);
    match expr {
        Expr::Number(num) => {
            let name = CString::new("value").unwrap();

            let bind_value_type = LLVMInt32TypeInContext(context);
            // Create stack space for i32
            let alloca = LLVMBuildAlloca(builder, bind_value_type, name.as_ptr());
            // Create constant `num`
            let bind_value =
                LLVMConstInt(bind_value_type, *num as c_ulonglong, LLVMBool::from(false));
            // Save constant in stack
            LLVMBuildStore(builder, bind_value, alloca);
            let bind_value_type = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
            // Cast stack address to *i8 (reuse previous i8 type)
            let i8_alloca =
                LLVMBuildPointerCast(builder, alloca, bind_value_type, EMPTY_STR.as_ptr());

            let bind_type_type = LLVMInt8TypeInContext(context);
            // Expr::Number identifier is 0
            let bind_type = LLVMConstInt(bind_type_type, 0, LLVMBool::from(false));
            (bind_type, i8_alloca)
        }
        Expr::Procedure(proc_name, args) => {
            let call_builder =
                ProcedureCallBuilder::new(runtime_ref, function_factory, module, builder);
            let (result_type_ptr, result_value) = call_builder.process_procedure(proc_name, args);
            // Push result to function args
            let context = LLVMGetModuleContext(module);
            let result_type_type = LLVMInt8TypeInContext(context);
            let result_type = LLVMBuildLoad2(
                builder,
                result_type_type,
                result_type_ptr,
                EMPTY_STR.as_ptr(),
            );
            (result_type, result_value)
        }
        _ => unimplemented!(),
    }
}
