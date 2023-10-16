use crate::backend::compilisp_ir::{Alloc, AllocId, AllocType};
use crate::backend::compilisp_llvm_generator::{
    CompilispLLVMGenerator, NUMBER_DISCRIMINATOR, STR_DISCRIMINATOR,
};
use crate::backend::error::CompilispResult;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::llvm_builder::Builder;
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
    module: LLVMModuleRef,
    alloc_map: &'a HashMap<AllocId, LLVMValueRef>,
    expr_builder: &'a CompilispLLVMGenerator<'a>,
}

impl<'a> ProcedureCallBuilder<'a> {
    pub fn new(
        function_factory: &'a FunctionFactory,
        type_factory: &'a TypeFactory,
        builder: LLVMBuilderRef,
        module: LLVMModuleRef,
        alloc_map: &'a HashMap<AllocId, LLVMValueRef>,
        expr_builder: &'a CompilispLLVMGenerator<'a>,
    ) -> Self {
        Self {
            function_factory,
            type_factory,
            builder,
            module,
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
        match name {
            "begin" | "+" | "display" | "<" => unsafe {
                Ok(self.procedure_runtime_call(name, args, return_alloc))
            },
            _ => unsafe { self.procedure_function_call(name, args, return_alloc) },
        }
    }

    unsafe fn procedure_function_call(
        &self,
        name: &str,
        args: &[Alloc],
        result_alloc: LLVMValueRef,
    ) -> CompilispResult<LLVMValueRef> {
        let c_name = CString::new(name).unwrap();
        let function = LLVMGetNamedFunction(self.module, c_name.as_ptr());
        let argc_type = self.type_factory.get_type(CompilispType::Int);
        let object_type = self.type_factory.get_type(CompilispType::CompilispObject);
        let obj_arr_type = self.type_factory.get_pointer(object_type);
        let mut args_types = [argc_type, obj_arr_type];
        let args_size = args_types.len() as c_uint;
        let args_ptr = args_types.as_mut_ptr();
        let fn_type = LLVMFunctionType(object_type, args_ptr, args_size, LLVMBool::from(false));

        let object_type = self.type_factory.get_type(CompilispType::CompilispObject);
        let object_array_type = LLVMArrayType(object_type, args.len() as _);
        let object_array =
            unsafe { LLVMBuildAlloca(self.builder, object_array_type, EMPTY_STR.as_ptr()) };

        let builder = Builder::new(self.builder);
        for (i, arg) in args.iter().enumerate() {
            let val_indexes = [0, i];
            let obj_type_indexes = [0, 0];

            let object_idx = builder.gep(object_array, object_array_type, &val_indexes);
            let type_attr_ptr = builder.gep(object_idx, object_type, &obj_type_indexes);

            let discriminator = match arg.alloc_type {
                AllocType::Int => Value::ConstInt(NUMBER_DISCRIMINATOR),
                AllocType::String => Value::ConstInt(STR_DISCRIMINATOR),
                AllocType::Bool => {
                    todo!()
                }
            };
            let value_discriminator = self.expr_builder.build_value(&discriminator);

            LLVMBuildStore(self.builder, value_discriminator, type_attr_ptr);
            // Copy Compilisp object value
            let value_ptr = *self.alloc_map.get(&arg.id).unwrap();
            let src_value_type = self.type_factory.get_type(CompilispType::CompilispObject);
            let src_value =
                LLVMBuildLoad2(self.builder, src_value_type, value_ptr, EMPTY_STR.as_ptr());
            LLVMBuildStore(self.builder, src_value, object_idx);
        }
        let object_array_ptr = builder.gep(object_array, object_array_type, &[0, 0]);

        let stack_size_value = self
            .expr_builder
            .build_value(&Value::ConstInt(args.len() as i32));

        let mut args = [stack_size_value, object_array_ptr];
        let result_value = LLVMBuildCall2(
            self.builder,
            fn_type,
            function,
            args.as_mut_ptr(),
            args_size,
            EMPTY_STR.as_ptr(),
        );
        LLVMBuildStore(self.builder, result_value, result_alloc);
        Ok(result_alloc)
    }

    unsafe fn procedure_runtime_call(
        &self,
        opname: &str,
        args: &[Alloc],
        result_alloc: LLVMValueRef,
    ) -> LLVMValueRef {
        let builder = Builder::new(self.builder);

        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_procedure_call")
            .copied()
            .unwrap();
        let stack_size_value = self
            .expr_builder
            .build_value(&Value::ConstInt(args.len() as i32));

        let object_type = self.type_factory.get_type(CompilispType::CompilispObject);
        let object_array_type = LLVMArrayType(object_type, args.len() as _);
        let object_array =
            unsafe { LLVMBuildAlloca(self.builder, object_array_type, EMPTY_STR.as_ptr()) };

        let name = "__operation_".to_string() + opname;
        let opname = self.expr_builder.build_value(&Value::GlobalString {
            value: opname,
            name: name.as_str(),
        });

        for (i, arg) in args.iter().enumerate() {
            let object_idx = builder.gep(object_array, object_array_type, &[0, i]);

            let type_attr_ptr = builder.gep(object_idx, object_type, &[0, 0]);

            let discriminator = match arg.alloc_type {
                AllocType::Int => Value::ConstInt(NUMBER_DISCRIMINATOR),
                AllocType::String => Value::ConstInt(STR_DISCRIMINATOR),
                AllocType::Bool => {
                    todo!()
                }
            };
            let value_discriminator = self.expr_builder.build_value(&discriminator);

            LLVMBuildStore(self.builder, value_discriminator, type_attr_ptr);
            // Copy Compilisp object value
            let value_ptr = *self.alloc_map.get(&arg.id).unwrap();
            let src_value_type = self.type_factory.get_type(CompilispType::CompilispObject);
            let src_value =
                LLVMBuildLoad2(self.builder, src_value_type, value_ptr, EMPTY_STR.as_ptr());
            LLVMBuildStore(self.builder, src_value, object_idx);
        }

        let object_array_ptr = builder.gep(object_array, object_array_type, &[0, 0]);

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
