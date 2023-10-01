use crate::backend::compilisp_ir::{Alloc, AllocId, AllocType};
use crate::backend::compilisp_llvm_generator::{
    CompilispLLVMGenerator, NUMBER_DISCRIMINATOR, STR_DISCRIMINATOR,
};
use crate::backend::error::CompilispResult;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::runtime::EMPTY_STR;
use crate::backend::type_factory::{CompilispType, TypeFactory};
use crate::backend::value_builder::Value;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::collections::HashMap;
use std::ffi::{c_uint, CString};

pub struct ProcedureCallBuilder<'a> {
    function_factory: &'a FunctionFactory,
    type_factory: &'a TypeFactory,
    builder: LLVMBuilderRef,
    alloc_map: &'a HashMap<AllocId, LLVMValueRef>,
    expr_builder: &'a CompilispLLVMGenerator<'a>,
}

impl<'a> ProcedureCallBuilder<'a> {
    pub fn new(
        function_factory: &'a FunctionFactory,
        type_factory: &'a TypeFactory,
        builder: LLVMBuilderRef,
        alloc_map: &'a HashMap<AllocId, LLVMValueRef>,
        expr_builder: &'a CompilispLLVMGenerator<'a>,
    ) -> Self {
        Self {
            function_factory,
            type_factory,
            builder,
            expr_builder,
            alloc_map,
        }
    }

    pub fn build_call(
        &self,
        name: &str,
        args: &[Alloc],
        return_alloc: LLVMValueRef,
    ) -> CompilispResult<LLVMValueRef> {
        unsafe { Ok(self.procedure_generic_call(name, args, return_alloc)) }
    }

    unsafe fn procedure_generic_call(
        &self,
        opname: &str,
        args: &[Alloc],
        result_alloc: LLVMValueRef,
    ) -> LLVMValueRef {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_procedure_call")
            .copied()
            .unwrap();
        let stack_size_value = self
            .expr_builder
            .build_value(&Value::ConstInt(args.len() as i32));

        let zero_idx = self.expr_builder.build_value(&Value::ConstInt(0));
        let obj_type_idx = self.expr_builder.build_value(&Value::ConstInt(0));

        let object_type = self.type_factory.get_type(CompilispType::CompilispObject);
        let object_array_type = LLVMArrayType(object_type, args.len() as _);
        let object_array =
            unsafe { LLVMBuildAlloca(self.builder, object_array_type, EMPTY_STR.as_ptr()) };

        let name = "__operation_".to_string() + opname;
        let opname = self.expr_builder.build_value(&Value::GlobalString {
            value: opname,
            name: name.as_str(),
        });

        for i in 0..args.len() {
            let index_value = self.expr_builder.build_value(&Value::ConstInt(i as i32));

            let mut val_indexes = [zero_idx, index_value];
            let mut obj_type_indexes = [zero_idx, obj_type_idx];

            let object_idx_ptr = val_indexes.as_mut_ptr();
            let obj_type_idx_ptr = obj_type_indexes.as_mut_ptr();
            let arg_str = CString::new("arg_obj").unwrap();
            let object_idx = LLVMBuildInBoundsGEP2(
                self.builder,
                object_array_type,
                object_array,
                object_idx_ptr,
                2,
                arg_str.as_ptr(),
            );

            let type_attr_ptr = LLVMBuildInBoundsGEP2(
                self.builder,
                object_type,
                object_idx,
                obj_type_idx_ptr,
                2,
                EMPTY_STR.as_ptr(),
            );
            let discriminator = match args[i].alloc_type {
                AllocType::Int => Value::ConstInt(NUMBER_DISCRIMINATOR),
                AllocType::String => Value::ConstInt(STR_DISCRIMINATOR),
                AllocType::Bool => {
                    todo!()
                }
            };
            let value_discriminator = self.expr_builder.build_value(&discriminator);

            LLVMBuildStore(self.builder, value_discriminator, type_attr_ptr);
            // Copy Compilisp object value
            let value_ptr = *self.alloc_map.get(&args[i].id).unwrap();
            let src_value_type = self.type_factory.get_type(CompilispType::CompilispObject);
            let src_value =
                LLVMBuildLoad2(self.builder, src_value_type, value_ptr, EMPTY_STR.as_ptr());
            LLVMBuildStore(self.builder, src_value, object_idx);
        }

        let mut first_idx = [zero_idx, zero_idx];

        let object_array_ptr = unsafe {
            LLVMBuildInBoundsGEP2(
                self.builder,
                object_array_type,
                object_array,
                first_idx.as_mut_ptr(),
                2,
                EMPTY_STR.as_ptr(),
            )
        };

        let mut args = [opname, object_array_ptr, stack_size_value];
        let res_name = CString::new("result").unwrap();
        let result_value = LLVMBuildCall2(
            self.builder,
            fn_argtypes,
            fn_ref,
            args.as_mut_ptr(),
            args.len() as c_uint,
            res_name.as_ptr(),
        );
        LLVMBuildStore(self.builder, result_value, result_alloc);
        result_alloc
    }
}
