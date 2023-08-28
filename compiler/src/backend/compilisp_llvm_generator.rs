use crate::backend::compilisp_ir::{AllocId, CompilispIr};
use crate::backend::function_factory::FunctionFactory;
use crate::backend::procedure_call_builder::ProcedureCallBuilder;
use crate::backend::runtime::{ELSE_STR, EMPTY_STR, FINALLY_STR, THEN_STR};
use crate::backend::value_builder::Value::GlobalString;
use crate::backend::value_builder::{Value, ValueBuilder};
use llvm_sys::core::*;
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBool, LLVMBuilderRef, LLVMModuleRef, LLVMValueRef};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_ulonglong;

pub const NUMBER_DISCRIMINATOR: i32 = 0;
pub const BOOLEAN_DISCRIMINATOR: c_ulonglong = 1;
pub const STR_DISCRIMINATOR: i32 = 2;
pub const SYMBOL_DISCRIMINATOR: i32 = 3;

struct ConditionalBlock {
    block_then: LLVMBasicBlockRef,
    block_else: Option<LLVMBasicBlockRef>,
    block_finally: LLVMBasicBlockRef,
}

pub struct CompilispLLVMGenerator<'a> {
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    runtime_ref: LLVMValueRef,
    value_builder: RefCell<ValueBuilder>,
    function_factory: &'a FunctionFactory,
    alloc_map: HashMap<AllocId, LLVMValueRef>,
    conditional_blocks: Vec<ConditionalBlock>,
}

impl<'a> CompilispLLVMGenerator<'a> {
    pub(crate) fn new(
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        runtime_ref: LLVMValueRef,
        function_factory: &'a FunctionFactory,
    ) -> Self {
        let value_builder = RefCell::new(ValueBuilder::default());
        let alloc_map = HashMap::new();
        let conditional_blocks = Vec::new();
        Self {
            module,
            builder,
            runtime_ref,
            value_builder,
            function_factory,
            alloc_map,
            conditional_blocks,
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
                    &self.alloc_map,
                    self,
                );

                let return_alloc = self.alloc_map.get(&return_id).unwrap();
                let result_type_ptr = call_builder
                    .build_call(name.as_str(), args.as_slice(), *return_alloc)
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
            CompilispIr::IfExpressionEval { cond_alloc } => unsafe {
                let context = LLVMGetModuleContext(self.module);
                let cond_value = self.alloc_map.get(&cond_alloc).unwrap();
                let ptr_to_bool = LLVMBuildPointerCast(
                    self.builder,
                    *cond_value,
                    LLVMPointerType(LLVMInt32TypeInContext(context), 0),
                    EMPTY_STR.as_ptr(),
                );
                let cond_value = LLVMBuildLoad2(
                    self.builder,
                    LLVMInt32TypeInContext(context),
                    ptr_to_bool,
                    EMPTY_STR.as_ptr(),
                );
                let cond_value_bool = LLVMBuildIntCast2(
                    self.builder,
                    cond_value,
                    LLVMInt1TypeInContext(context),
                    LLVMBool::from(false),
                    EMPTY_STR.as_ptr(),
                );

                let block_then = LLVMCreateBasicBlockInContext(context, THEN_STR.as_ptr());
                let block_else = LLVMCreateBasicBlockInContext(context, ELSE_STR.as_ptr());
                let block_finally = LLVMCreateBasicBlockInContext(context, FINALLY_STR.as_ptr());
                self.conditional_blocks.push(ConditionalBlock {
                    block_else: Some(block_else),
                    block_then,
                    block_finally,
                });
                LLVMBuildCondBr(self.builder, cond_value_bool, block_then, block_else);
                LLVMInsertExistingBasicBlockAfterInsertBlock(self.builder, block_then);
                LLVMPositionBuilderAtEnd(self.builder, block_then);
            },
            CompilispIr::IfExpressionEndThen | CompilispIr::IfExpressionEndElse => unsafe {
                let cur_block = self.conditional_blocks.last().unwrap();
                if cur_block.block_else.is_some() {
                    LLVMBuildBr(self.builder, cur_block.block_finally);
                }
            },
            CompilispIr::IfExpressionElse { .. } => unsafe {
                let cur_block = self.conditional_blocks.last().unwrap();
                if let Some(block_else) = cur_block.block_else {
                    LLVMInsertExistingBasicBlockAfterInsertBlock(self.builder, block_else);
                    LLVMPositionBuilderAtEnd(self.builder, block_else);
                }
            },
            CompilispIr::IfExpressionEndBlock { .. } => {
                let cur_block = self.conditional_blocks.last().unwrap();
                unsafe {
                    LLVMInsertExistingBasicBlockAfterInsertBlock(
                        self.builder,
                        cur_block.block_finally,
                    );
                    LLVMPositionBuilderAtEnd(self.builder, cur_block.block_finally);
                }
            }
            CompilispIr::IfExpressionEndExpression { .. } => {}
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
