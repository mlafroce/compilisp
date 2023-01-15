use std::collections::HashMap;
use std::ffi::{c_uint, c_ulonglong, CString};
use std::ptr::null_mut;
use llvm_sys::core::{LLVMBuildAlloca, LLVMBuildCall2, LLVMBuildGlobalStringPtr, LLVMBuildPointerCast, LLVMBuildStore, LLVMConstInt, LLVMGetModuleContext, LLVMInt32TypeInContext, LLVMInt8TypeInContext, LLVMPointerType};
use llvm_sys::prelude::{LLVMBool, LLVMBuilderRef, LLVMContextRef, LLVMModuleRef, LLVMTypeRef, LLVMValueRef};
use crate::ast::Expr;
use crate::backend::function::FunctionBuilder;

pub struct RuntimeCompiler {
    runtime_ref: LLVMValueRef,
    function_factory: FunctionFactory
}

impl RuntimeCompiler {
    pub unsafe fn init(
        builder: LLVMBuilderRef,
        function_factory: FunctionFactory,
    ) -> Self {
        let (fn_ref, fn_argtypes) = function_factory.get("compilisp_init").copied().unwrap();
        let call_str = CString::new("").unwrap();
        let runtime_ref = LLVMBuildCall2(builder, fn_argtypes, fn_ref, null_mut(), 0, call_str.as_ptr());
        Self { runtime_ref, function_factory }
    }

    pub unsafe fn destroy(
        self,
        builder: LLVMBuilderRef,
    ) {
        let (fn_ref, fn_argtypes) = self.function_factory.get("compilisp_destroy").copied().unwrap();

        let mut args = [self.runtime_ref];
        let call_str = CString::new("").unwrap();
        LLVMBuildCall2(builder, fn_argtypes, fn_ref, args.as_mut_ptr(), 1, call_str.as_ptr());
    }

    pub unsafe fn process_expr(
        &self,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        expression: &Expr,
    ) {
        match expression {
            Expr::LetProcedure(bindings, expression) => {
                self.process_let(module, builder, bindings, expression.as_ref());
            }
            Expr::Procedure(name, args) => {
                self.process_procedure(module, builder, name, args);
            }
            _ => {
                println!("Cannot process this token yet")
            }
        }
    }

    unsafe fn process_procedure(
        &self,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        name: &str,
        args: &Vec<Expr>,
    ) {
        unimplemented!();
    }

    unsafe fn process_let(
        &self,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        bindings: &Vec<(String, Expr)>,
        expression: &Expr,
    ) {
        let context = LLVMGetModuleContext(module);
        let (fn_ref, fn_argtypes) = self.function_factory.get("compilisp_push_let_context").copied().unwrap();

        let mut args = [self.runtime_ref];
        let call_str = CString::new("").unwrap();
        LLVMBuildCall2(builder, fn_argtypes, fn_ref, args.as_mut_ptr(), 1, call_str.as_ptr());

        for (binding_name, binding_expr) in bindings {
            self.bind_let_value(context, builder, &binding_name, binding_expr);
        }
        self.process_expr(module, builder, expression);
    }

    unsafe fn bind_let_value(&self, context: LLVMContextRef, builder: LLVMBuilderRef, binding_name: &str, binding_expr: &Expr) {
        let (fn_ref, fn_argtypes) = self.function_factory.get("compilisp_push_let_binding").copied().unwrap();
        let c_binding_name = CString::new(binding_name).unwrap();
        let empty_str = CString::new("").unwrap();
        let name_value = LLVMBuildGlobalStringPtr(builder, c_binding_name.as_ptr(), empty_str.as_ptr());

        let (bind_type, bind_value) = match binding_expr {
            Expr::Number(num) => {
                let name = CString::new("binding_value").unwrap();

                let bind_value_type = LLVMInt32TypeInContext(context);
                // Create stack space for i32
                let alloca = LLVMBuildAlloca(builder, bind_value_type, name.as_ptr());
                // Create constant `num`
                let bind_value = LLVMConstInt(bind_value_type, *num as c_ulonglong, LLVMBool::from(false));
                // Save constant in stack
                LLVMBuildStore(builder, bind_value, alloca);
                let bind_value_type = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
                // Cast stack address to *i8 (reuse previous i8 type)
                let i8_alloca = LLVMBuildPointerCast(builder, alloca, bind_value_type, empty_str.as_ptr());

                let bind_type_type = LLVMInt8TypeInContext(context);
                // Expr::Number identifier is 0
                let bind_type = LLVMConstInt(bind_type_type, 0, LLVMBool::from(false));
                (bind_type, i8_alloca)
            }
            _ => unimplemented!(),
        };
        let mut args = [self.runtime_ref, name_value, bind_type, bind_value];
        let call_str = CString::new("").unwrap();
        LLVMBuildCall2(builder, fn_argtypes, fn_ref, args.as_mut_ptr(), args.len() as c_uint, call_str.as_ptr());
    }
}

pub struct FunctionFactory {
    function_map: HashMap<String, (LLVMValueRef, LLVMTypeRef)>
}

impl FunctionFactory {
    /// Creates a FunctionFactory with some base methods
    /// #Safety
    /// Must be a module in a context
    pub unsafe fn new_with_base(module: LLVMModuleRef) ->  Self {
        let mut function_map = HashMap::new();
        let context = LLVMGetModuleContext(module);
        let char_type = LLVMInt8TypeInContext(context);
        let opaque_pointer = LLVMPointerType(char_type, 0);

        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_init")
            .with_ret_type(opaque_pointer);
        let cur_fn = fn_builder.build(module);
        function_map.insert("compilisp_init".to_owned(), cur_fn);

        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_destroy")
            .add_arg(opaque_pointer);
        let cur_fn = fn_builder.build(module);
        function_map.insert("compilisp_destroy".to_owned(), cur_fn);

        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_push_let_context")
            .add_arg(opaque_pointer);
        let cur_fn = fn_builder.build(module);
        function_map.insert("compilisp_push_let_context".to_owned(), cur_fn);

        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_pop_let_context")
            .add_arg(opaque_pointer);
        let cur_fn = fn_builder.build(module);
        function_map.insert("compilisp_pop_let_context".to_owned(), cur_fn);


        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_push_let_binding")
            .add_arg(opaque_pointer)// context
            .add_arg(opaque_pointer)// name
            .add_arg(char_type)// expr type
            .add_arg(opaque_pointer); // expr value
        let cur_fn = fn_builder.build(module);
        function_map.insert("compilisp_push_let_binding".to_owned(), cur_fn);

        Self { function_map }
    }

    pub fn get(&self, name: &str) -> Option<&(LLVMValueRef, LLVMTypeRef)> {
        self.function_map.get(name)
    }
}