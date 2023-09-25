use crate::backend::function_builder::FunctionBuilder;
use crate::backend::type_factory::{CompilispType, TypeFactory};
use llvm_sys::prelude::{LLVMModuleRef, LLVMTypeRef, LLVMValueRef};
use std::collections::HashMap;

// TODO: this shouldn't be clonable
#[derive(Clone)]
pub struct FunctionFactory {
    function_map: HashMap<String, (LLVMValueRef, LLVMTypeRef)>,
}

impl FunctionFactory {
    /// Creates a FunctionFactory with some base methods
    /// #Safety
    /// Must be a module in a context
    pub fn new_with_base(module: LLVMModuleRef, type_factory: &TypeFactory) -> Self {
        let mut function_map = HashMap::new();
        let int_type = type_factory.get_type(CompilispType::Int);
        let char_pointer = type_factory.get_type(CompilispType::CharPtr);
        let object_type = type_factory.get_type(CompilispType::CompilispObject);
        let object_pointer = type_factory.get_pointer(object_type);

        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_init")
            .with_ret_type(char_pointer);
        let cur_fn = unsafe { fn_builder.build(module) };
        function_map.insert("compilisp_init".to_owned(), cur_fn);

        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_destroy")
            .add_arg(char_pointer);
        let cur_fn = unsafe { fn_builder.build(module) };
        function_map.insert("compilisp_destroy".to_owned(), cur_fn);

        let fn_builder = FunctionBuilder::new()
            .with_name("compilisp_procedure_call")
            .with_ret_type(object_type)
            .add_arg(char_pointer) // name
            .add_arg(object_pointer) // result type
            .add_arg(int_type); // args size
        let cur_fn = unsafe { fn_builder.build(module) };
        function_map.insert("compilisp_procedure_call".to_owned(), cur_fn);

        Self { function_map }
    }

    pub fn get(&self, name: &str) -> Option<&(LLVMValueRef, LLVMTypeRef)> {
        self.function_map.get(name)
    }
}
