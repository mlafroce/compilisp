use std::cell::RefCell;
use crate::ast::Expr;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::procedure_call_builder::ProcedureCallBuilder;
use crate::backend::runtime::EMPTY_STR;
use crate::backend::value_builder::Value::{ConstInt, GlobalString};
use crate::backend::value_builder::{Value, ValueBuilder};
use llvm_sys::core::*;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMModuleRef, LLVMValueRef};
use std::ffi::{c_uint, c_ulonglong, CString};

pub const NUMBER_DISCRIMINATOR: i32 = 0;
pub const BOOLEAN_DISCRIMINATOR: c_ulonglong = 1;
pub const STR_DISCRIMINATOR: i32 = 2;
pub const SYMBOL_DISCRIMINATOR: i32 = 3;

pub struct ExprBuilder<'a> {
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    runtime_ref: LLVMValueRef,
    value_builder: RefCell<ValueBuilder>,
    function_factory: &'a FunctionFactory,
}

impl<'a> ExprBuilder<'a> {
    pub(crate) fn new(
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        runtime_ref: LLVMValueRef,
        function_factory: &'a FunctionFactory,
    ) -> Self {
        let value_builder = RefCell::new(ValueBuilder::default());
        Self {
            module,
            builder,
            runtime_ref,
            value_builder,
            function_factory,
        }
    }

    pub fn build_value(&self, value: &Value) -> LLVMValueRef {
        let context = unsafe { LLVMGetModuleContext(self.module) };
        let mut value_builder = self.value_builder.borrow_mut();
        unsafe { value_builder.build_value(context, self.builder, value) }
    }

    pub fn build_expr(&self, expression: &Expr) -> (LLVMValueRef, LLVMValueRef) {
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
                    self,
                );
                call_builder.process_procedure(name, args).unwrap()
            }
            _ => {
                unimplemented!("Cannot process this token yet {:?}", expression)
            }
        }
    }

    fn process_let(
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
        unsafe {
            LLVMBuildCall2(
                self.builder,
                fn_argtypes,
                fn_ref,
                args.as_mut_ptr(),
                1,
                EMPTY_STR.as_ptr(),
            );
        }

        for (binding_name, binding_expr) in bindings {
            self.bind_let_value(binding_name, binding_expr);
        }
        self.build_expr(expression)
    }

    fn bind_let_value(&self, binding_name: &str, binding_expr: &Expr) {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_push_let_binding")
            .copied()
            .unwrap();
        let c_binding_name = CString::new(binding_name).unwrap();
        let name_value = unsafe {
            LLVMBuildGlobalStringPtr(self.builder, c_binding_name.as_ptr(), EMPTY_STR.as_ptr())
        };

        let (bind_type, bind_value) = self.build_expr_in_stack(binding_expr);
        let mut args = [self.runtime_ref, name_value, bind_type, bind_value];
        unsafe {
            LLVMBuildCall2(
                self.builder,
                fn_argtypes,
                fn_ref,
                args.as_mut_ptr(),
                args.len() as c_uint,
                EMPTY_STR.as_ptr(),
            );
        }
    }

    /// Returns a tuple with (expr_type, expr_value) pointers
    pub fn build_expr_in_stack(&self, expr: &Expr) -> (LLVMValueRef, LLVMValueRef) {
        let context = unsafe { LLVMGetModuleContext(self.module) };
        match expr {
            Expr::Number(num) => self.build_number_in_stack(*num),
            Expr::Boolean(value) => self.build_boolean_in_stack(*value),
            Expr::Symbol(name) => self.build_symbol_in_stack(name.as_str()),
            Expr::String(value) => self.build_str_in_stack(value.as_str()),
            Expr::Procedure(proc_name, args) => {
                let call_builder = ProcedureCallBuilder::new(
                    self.runtime_ref,
                    self.function_factory,
                    self.module,
                    self.builder,
                    self,
                );
                let (result_type_ptr, result_value) =
                    call_builder.process_procedure(proc_name, args).unwrap();

                let result_type_type = unsafe { LLVMInt8TypeInContext(context) };
                let result_type = unsafe {
                    LLVMBuildLoad2(
                        self.builder,
                        result_type_type,
                        result_type_ptr,
                        EMPTY_STR.as_ptr(),
                    )
                };
                (result_type, result_value)
            }
            _ => unimplemented!(),
        }
    }

    fn build_str_in_stack(
        &self,
        value: &str,
    ) -> (LLVMValueRef, LLVMValueRef) {
        let symbol = GlobalString {
            name: "static_str",
            value,
        };
        let str_ptr = self.build_value(&symbol);

        let discriminator = ConstInt(STR_DISCRIMINATOR);
        let value_discriminator = self.build_value(&discriminator);
        (value_discriminator, str_ptr)
    }

    fn build_symbol_in_stack(
        &self,
        sym_name: &str,
    ) -> (LLVMValueRef, LLVMValueRef) {
        let symbol = GlobalString {
            name: "symbol_name",
            value: sym_name,
        };
        let symbol_ptr = self.build_value(&symbol);

        let discriminator = ConstInt(SYMBOL_DISCRIMINATOR);
        let value_discriminator = self.build_value(&discriminator);
        (value_discriminator, symbol_ptr)
    }

    fn build_number_in_stack(
        &self,
        num: i32,
    ) -> (LLVMValueRef, LLVMValueRef) {
        let context = unsafe { LLVMGetModuleContext(self.module) };

        let value = Value::VarInt32("value", Some(num));
        let value_ptr = self.build_value(&value);
        let value_opaque = unsafe { ValueBuilder::cast_opaque(context, self.builder, &value_ptr) };

        let discriminator = ConstInt(NUMBER_DISCRIMINATOR);
        let value_discriminator = self.build_value(&discriminator);

        (value_discriminator, value_opaque)
    }
    fn build_boolean_in_stack(
        &self,
        value: bool,
    ) -> (LLVMValueRef, LLVMValueRef) {
        let context = unsafe { LLVMGetModuleContext(self.module) };

        let value = Value::VarBool("value", Some(value));
        let value_ptr = self.build_value(&value);
        let value_opaque = unsafe { ValueBuilder::cast_opaque(context, self.builder, &value_ptr) };

        let discriminator = ConstInt(NUMBER_DISCRIMINATOR);
        let value_discriminator = self.build_value(&discriminator);

        (value_discriminator, value_opaque)
    }
}
