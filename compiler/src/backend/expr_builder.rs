use crate::backend::compilisp_ir::{AllocId, AllocType, CompilispIr};
use crate::backend::function_factory::FunctionFactory;
use crate::backend::procedure_call_builder::ProcedureCallBuilder;
use crate::backend::runtime::EMPTY_STR;
use crate::backend::value_builder::Value::{ConstInt, GlobalString};
use crate::backend::value_builder::{Value, ValueBuilder};
use llvm_sys::core::*;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMModuleRef, LLVMValueRef};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_ulonglong;

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
    alloc_map: HashMap<AllocId, LLVMValueRef>,
}

impl<'a> ExprBuilder<'a> {
    pub(crate) fn new(
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        runtime_ref: LLVMValueRef,
        function_factory: &'a FunctionFactory,
    ) -> Self {
        let value_builder = RefCell::new(ValueBuilder::default());
        let alloc_map = HashMap::new();
        Self {
            module,
            builder,
            runtime_ref,
            value_builder,
            function_factory,
            alloc_map,
        }
    }

    pub fn build_value(&self, value: &Value) -> LLVMValueRef {
        let context = unsafe { LLVMGetModuleContext(self.module) };
        let mut value_builder = self.value_builder.borrow_mut();
        unsafe { value_builder.build_value(context, self.builder, value) }
    }
    pub fn build_instruction(&mut self, inst: CompilispIr) {
        match inst {
            CompilispIr::ConstInt { alloc_id, value } => {
                let builder_value = Value::VarInt32("", Some(value));
                let alloc = self.build_value(&builder_value);
                self.alloc_map.insert(alloc_id, alloc);
            }
            CompilispIr::GlobalString { alloc_id, value } => {
                let symbol = GlobalString {
                    name: "symbol_name",
                    value: value.as_str(),
                };
                let alloc = self.build_value(&symbol);
                self.alloc_map.insert(alloc_id, alloc);
            }
            CompilispIr::CallProcedure {
                name,
                args,
                return_id,
            } => {
                let context = unsafe { LLVMGetModuleContext(self.module) };
                let call_builder = ProcedureCallBuilder::new(
                    self.runtime_ref,
                    self.function_factory,
                    self.module,
                    self.builder,
                    self,
                );
                let args_alloc = args
                    .iter()
                    .flat_map(|arg| {
                        self.alloc_map
                            .get(&arg.id)
                            .map(|alloc| (arg.alloc_type, alloc))
                    })
                    .map(|(alloc_type, alloc)| {
                        let discriminator = match alloc_type {
                            AllocType::Int => ConstInt(NUMBER_DISCRIMINATOR),
                            AllocType::String => ConstInt(STR_DISCRIMINATOR),
                            AllocType::Bool => {
                                todo!()
                            }
                        };
                        let value_discriminator = self.build_value(&discriminator);
                        (value_discriminator, alloc)
                    })
                    .map(|(disc, value)| unsafe {
                        let opaque_ptr_t = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
                        let casted = LLVMBuildBitCast(
                            self.builder,
                            *value,
                            opaque_ptr_t,
                            EMPTY_STR.as_ptr(),
                        );
                        (disc, casted)
                    })
                    .collect::<Vec<_>>();

                let return_alloc = self.alloc_map.get(&return_id).unwrap();
                let (result_type_ptr, _) = call_builder
                    .build_generic_call(name.as_str(), args_alloc.as_slice(), *return_alloc)
                    .unwrap();
                let result_type_type = unsafe { LLVMInt8TypeInContext(context) };
                let _ = unsafe {
                    LLVMBuildLoad2(
                        self.builder,
                        result_type_type,
                        result_type_ptr,
                        EMPTY_STR.as_ptr(),
                    )
                };
            }
            CompilispIr::ProcedureScopeStart => {}
            CompilispIr::ProcedureScopeEnd => {}
            // Same as allocVar
            CompilispIr::ProcedureReturnValue(id) => {
                let value = Value::VarInt32("", None);
                let alloc = self.build_value(&value);
                self.alloc_map.insert(id, alloc);
            }
            CompilispIr::PushArg(_) => {}
        }
    }
}
