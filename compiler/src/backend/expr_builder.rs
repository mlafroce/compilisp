use std::ffi::{c_uint, c_ulonglong, CString};
use llvm_sys::core::*;
use llvm_sys::prelude::{LLVMBool, LLVMBuilderRef, LLVMContextRef, LLVMModuleRef, LLVMValueRef};
use crate::ast::Expr;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::procedure_call_builder::ProcedureCallBuilder;
use crate::backend::runtime::EMPTY_STR;

pub const NUMBER_DISCRIMINATOR: c_ulonglong = 0;
pub const STR_DISCRIMINATOR: c_ulonglong = 1;
pub const SYMBOL_DISCRIMINATOR: c_ulonglong = 2;


pub struct ExprBuilder<'a> {
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    runtime_ref: LLVMValueRef,
    function_factory: &'a FunctionFactory
}

impl<'a> ExprBuilder<'a> {
    pub(crate) fn new(module: LLVMModuleRef,
                      builder: LLVMBuilderRef,
                      runtime_ref: LLVMValueRef,
                      function_factory: &'a FunctionFactory
    ) -> Self {
        Self {module, builder, runtime_ref, function_factory}
    }

    pub unsafe fn build_expr(&self, expression: &Expr) -> (LLVMValueRef, LLVMValueRef) {
        match expression {
            Expr::LetProcedure(bindings, expression) => {
                self.process_let(bindings, expression.as_ref())
            }
            Expr::Procedure(name, args) => {
                let call_builder = ProcedureCallBuilder::new(
                    self.runtime_ref,
                    self.function_factory,
                    self.module,
                    self.builder,
                    self
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
            self.builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            1,
            EMPTY_STR.as_ptr(),
        );

        for (binding_name, binding_expr) in bindings {
            self.bind_let_value(binding_name, binding_expr);
        }
        self.build_expr(expression)
    }

    unsafe fn bind_let_value(
        &self,
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
            LLVMBuildGlobalStringPtr(self.builder, c_binding_name.as_ptr(), EMPTY_STR.as_ptr());

        let (bind_type, bind_value) = self.build_expr_in_stack(
            binding_expr,
        );
        let mut args = [self.runtime_ref, name_value, bind_type, bind_value];
        LLVMBuildCall2(
            self.builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            args.len() as c_uint,
            EMPTY_STR.as_ptr(),
        );
    }

    /// Returns a tuple with (expr_type, expr_value) pointers
    pub unsafe fn build_expr_in_stack(
        &self,
        expr: &Expr,
    ) -> (LLVMValueRef, LLVMValueRef) {
        let context = LLVMGetModuleContext(self.module);
        match expr {
            Expr::Number(num) => build_number_in_stack(context, self.builder, *num),
            Expr::Symbol(name) => build_symbol_in_stack(context, self.builder, name.as_str()),
            Expr::String(value) => build_str_in_stack(context, self.builder, value.as_str()),
            Expr::Procedure(proc_name, args) => {
                let call_builder =
                    ProcedureCallBuilder::new(self.runtime_ref, self.function_factory, self.module, self.builder, self);
                let (result_type_ptr, result_value) = call_builder.process_procedure(proc_name, args);
                // Push result to function args
                let context = LLVMGetModuleContext(self.module);
                let result_type_type = LLVMInt8TypeInContext(context);
                let result_type = LLVMBuildLoad2(
                    self.builder,
                    result_type_type,
                    result_type_ptr,
                    EMPTY_STR.as_ptr(),
                );
                (result_type, result_value)
            }
            _ => unimplemented!(),
        }
    }
}

unsafe fn build_symbol_in_stack(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    name: &str,
) -> (LLVMValueRef, LLVMValueRef) {
    let c_name = CString::new(name).unwrap();
    let c_name_var = CString::new("symbol_name").unwrap();
    let name_ptr = LLVMBuildGlobalStringPtr(builder, c_name.as_ptr(), c_name_var.as_ptr());

    let bind_type_type = LLVMInt8TypeInContext(context);
    // Expr::Number identifier is 0
    let value_discriminator = LLVMConstInt(bind_type_type, SYMBOL_DISCRIMINATOR, LLVMBool::from(false));
    (value_discriminator, name_ptr)
}

unsafe fn build_str_in_stack(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    name: &str,
) -> (LLVMValueRef, LLVMValueRef) {
    let c_name = CString::new(name).unwrap();
    let c_name_var = CString::new("str").unwrap();
    let name_ptr = LLVMBuildGlobalStringPtr(builder, c_name.as_ptr(), c_name_var.as_ptr());

    let bind_type_type = LLVMInt8TypeInContext(context);
    // Expr::Number identifier is 0
    let value_discriminator = LLVMConstInt(bind_type_type, STR_DISCRIMINATOR, LLVMBool::from(false));
    (value_discriminator, name_ptr)
}

unsafe fn build_number_in_stack(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    num: i32,
) -> (LLVMValueRef, LLVMValueRef) {
    let name = CString::new("value").unwrap();
    let bind_value_type = LLVMInt32TypeInContext(context);
    // Create stack space for i32
    let alloca = LLVMBuildAlloca(builder, bind_value_type, name.as_ptr());
    // Create constant `num`
    let bind_value = LLVMConstInt(bind_value_type, num as c_ulonglong, LLVMBool::from(false));
    // Save constant in stack
    LLVMBuildStore(builder, bind_value, alloca);
    let bind_value_type = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
    // Cast stack address to *i8 (reuse previous i8 type)
    let i8_alloca = LLVMBuildPointerCast(builder, alloca, bind_value_type, EMPTY_STR.as_ptr());

    let bind_type_type = LLVMInt8TypeInContext(context);
    // Expr::Number identifier is 0
    let value_discriminator = LLVMConstInt(bind_type_type, NUMBER_DISCRIMINATOR, LLVMBool::from(false));
    (value_discriminator, i8_alloca)
}
