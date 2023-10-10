use crate::backend::compilisp_ir::{AllocId, CompilispIr};
use crate::backend::function_builder::FunctionBuilder;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::procedure_call_builder::ProcedureCallBuilder;
use crate::backend::runtime::{ELSE_STR, EMPTY_STR, FINALLY_STR, THEN_STR};
use crate::backend::type_factory::{CompilispType, TypeFactory};
use crate::backend::value_builder::Value::VariableString;
use crate::backend::value_builder::{Value, ValueBuilder};
use llvm_sys::core::*;
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMModuleRef, LLVMValueRef};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_ulonglong;
use std::ptr::null_mut;

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
    value_builder: RefCell<ValueBuilder>,
    function_factory: &'a FunctionFactory,
    type_factory: &'a TypeFactory,
    alloc_map: HashMap<AllocId, LLVMValueRef>,
    conditional_blocks: Vec<ConditionalBlock>,
    current_function: Option<LLVMValueRef>,
}

impl<'a> CompilispLLVMGenerator<'a> {
    pub(crate) fn new(
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        function_factory: &'a FunctionFactory,
        type_factory: &'a TypeFactory,
    ) -> Self {
        let value_builder = RefCell::new(ValueBuilder::default());
        let alloc_map = HashMap::new();
        let conditional_blocks = Vec::new();
        let current_function = None;
        Self {
            module,
            builder,
            value_builder,
            function_factory,
            type_factory,
            alloc_map,
            conditional_blocks,
            current_function,
        }
    }

    pub fn build_value(&self, value: &Value) -> LLVMValueRef {
        let context = unsafe { LLVMGetModuleContext(self.module) };
        let mut value_builder = self.value_builder.borrow_mut();
        unsafe { value_builder.build_value(context, self.builder, value, self.type_factory) }
    }
    pub fn build_instruction(&mut self, inst: CompilispIr) {
        match inst {
            CompilispIr::ConstInt { alloc_id, value } => {
                let builder_value = Value::VarInt32("", Some(value));
                let alloc = self.build_value(&builder_value);
                self.alloc_map.insert(alloc_id, alloc);
            }
            CompilispIr::GlobalString { alloc_id, value } => {
                let symbol = VariableString {
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
                let call_builder = ProcedureCallBuilder::new(
                    self.function_factory,
                    self.type_factory,
                    self.builder,
                    self.module,
                    &self.alloc_map,
                    self,
                );

                let return_alloc = self.alloc_map.get(&return_id).unwrap();
                call_builder
                    .build_call(name.as_str(), args.as_slice(), *return_alloc)
                    .unwrap();
            }
            CompilispIr::IfExpressionEval { cond_alloc } => unsafe {
                let context = LLVMGetModuleContext(self.module);
                let cond_value = self.alloc_map.get(&cond_alloc).unwrap();
                let mut value_idx_ptr = [
                    self.build_value(&Value::ConstInt(0)),
                    self.build_value(&Value::ConstInt(1)),
                ];
                let value_attr_ptr = LLVMBuildInBoundsGEP2(
                    self.builder,
                    self.type_factory.get_type(CompilispType::CompilispObject),
                    *cond_value,
                    value_idx_ptr.as_mut_ptr(),
                    2,
                    EMPTY_STR.as_ptr(),
                );
                let int_type = self.type_factory.get_type(CompilispType::BoolPtr);
                let casted =
                    LLVMBuildBitCast(self.builder, value_attr_ptr, int_type, EMPTY_STR.as_ptr());

                let cond_value = LLVMBuildLoad2(
                    self.builder,
                    LLVMInt1TypeInContext(context),
                    casted,
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
                LLVMBuildCondBr(self.builder, cond_value, block_then, block_else);
                LLVMInsertExistingBasicBlockAfterInsertBlock(self.builder, block_then);
                LLVMPositionBuilderAtEnd(self.builder, block_then);
            },
            CompilispIr::IfExpressionEndThen {
                result_alloc,
                cond_alloc,
            }
            | CompilispIr::IfExpressionEndElse {
                result_alloc,
                cond_alloc,
            } => unsafe {
                // Copy block result into conditional result
                let result_value = self.alloc_map.get(&result_alloc).unwrap();
                let cond_value = self.alloc_map.get(&cond_alloc).unwrap();
                let result = LLVMBuildLoad2(
                    self.builder,
                    self.type_factory.get_type(CompilispType::CompilispObject),
                    *result_value,
                    EMPTY_STR.as_ptr(),
                );
                LLVMBuildStore(self.builder, result, *cond_value);
                let cur_block = self.conditional_blocks.last().unwrap();
                if cur_block.block_else.is_some() {
                    LLVMBuildBr(self.builder, cur_block.block_finally);
                }
            },
            CompilispIr::IfExpressionElse => unsafe {
                let cur_block = self.conditional_blocks.last().unwrap();
                if let Some(block_else) = cur_block.block_else {
                    LLVMInsertExistingBasicBlockAfterInsertBlock(self.builder, block_else);
                    LLVMPositionBuilderAtEnd(self.builder, block_else);
                }
            },
            CompilispIr::IfExpressionEndBlock => {
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
            CompilispIr::StartProcedure(name) => {
                let context = unsafe { LLVMGetModuleContext(self.module) };
                let int_type = self.type_factory.get_type(CompilispType::Int);
                let obj_type = self.type_factory.get_type(CompilispType::CompilispObject);
                let obj_ptr_type = self.type_factory.get_pointer(obj_type);
                let builder = FunctionBuilder::new()
                    .with_name(name.as_str())
                    .with_ret_type(obj_type)
                    .add_arg(int_type)
                    .add_arg(obj_ptr_type);
                let new_function = unsafe { builder.build(self.module) };
                let block = unsafe {
                    LLVMAppendBasicBlockInContext(context, new_function.0, EMPTY_STR.as_ptr())
                };
                self.current_function = Some(new_function.0);
                unsafe { LLVMPositionBuilderAtEnd(self.builder, block) };
            }
            CompilispIr::MapProcedureArgs(args, mut alloc_id) => {
                let fun = self.current_function.unwrap();
                let argc = unsafe { LLVMGetFirstParam(fun) };
                let argv = unsafe { LLVMGetNextParam(argc) };

                for i in 0..args.len() {
                    let mut value_idx_ptr = [self.build_value(&Value::ConstInt(i as i32))];
                    let cur_arg = unsafe {
                        LLVMBuildInBoundsGEP2(
                            self.builder,
                            self.type_factory.get_type(CompilispType::CompilispObject),
                            argv,
                            value_idx_ptr.as_mut_ptr(),
                            1,
                            EMPTY_STR.as_ptr(),
                        )
                    };

                    alloc_id += 1;
                    self.alloc_map.insert(alloc_id, cur_arg);
                }
            }
            CompilispIr::EndProcedure(return_id) => {
                let return_alloc = self.alloc_map.get(&return_id).unwrap();
                let value = unsafe {
                    LLVMBuildLoad2(
                        self.builder,
                        self.type_factory.get_type(CompilispType::CompilispObject),
                        *return_alloc,
                        EMPTY_STR.as_ptr(),
                    )
                };
                unsafe { LLVMBuildRet(self.builder, value) };
            }
        }
    }
}
