use crate::backend::compilisp_ir::{Alloc, AllocId, AllocType};
use crate::backend::compilisp_llvm_generator::{
    CompilispLLVMGenerator, NUMBER_DISCRIMINATOR, STR_DISCRIMINATOR,
};
use crate::backend::error::CompilispResult;
use crate::backend::function_factory::FunctionFactory;
use crate::backend::runtime::EMPTY_STR;
use crate::backend::value_builder::Value;
use crate::backend::value_builder::Value::ConstInt;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use std::collections::HashMap;
use std::ffi::{c_uint, c_ulonglong, CString};

pub struct ProcedureCallBuilder<'a> {
    runtime_ref: LLVMValueRef,
    function_factory: &'a FunctionFactory,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    alloc_map: &'a HashMap<AllocId, LLVMValueRef>,
    expr_builder: &'a CompilispLLVMGenerator<'a>,
}

impl<'a> ProcedureCallBuilder<'a> {
    pub fn new(
        runtime_ref: LLVMValueRef,
        function_factory: &'a FunctionFactory,
        module: LLVMModuleRef,
        builder: LLVMBuilderRef,
        alloc_map: &'a HashMap<AllocId, LLVMValueRef>,
        expr_builder: &'a CompilispLLVMGenerator,
    ) -> Self {
        Self {
            runtime_ref,
            function_factory,
            module,
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
        self.push_args(args);
        unsafe { Ok(self.procedure_generic_call(name, args.len(), return_alloc)) }
    }

    fn procedure_generic_push_arg(&self, arg_type: LLVMValueRef, arg_value: LLVMValueRef) {
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_procedure_push_arg")
            .copied()
            .unwrap();

        let mut args = [self.runtime_ref, arg_type, arg_value];
        unsafe {
            LLVMBuildCall2(
                self.builder,
                fn_argtypes,
                fn_ref,
                args.as_mut_ptr(),
                args.len() as c_uint,
                EMPTY_STR.as_ptr(),
            )
        };
    }

    unsafe fn procedure_generic_call(
        &self,
        name: &str,
        stack_size: usize,
        result_alloc: LLVMValueRef,
    ) -> LLVMValueRef {
        let context = LLVMGetModuleContext(self.module);
        let (fn_ref, fn_argtypes) = self
            .function_factory
            .get("compilisp_procedure_call")
            .copied()
            .unwrap();
        let call_name = "__gen_call_".to_owned() + name;
        let procedure_name = Value::GlobalString {
            value: name,
            name: call_name.as_str(),
        };
        let name_value = self.expr_builder.build_value(&procedure_name);

        let bind_type_type = LLVMInt8TypeInContext(context);
        let stack_size_value = LLVMConstInt(
            bind_type_type,
            stack_size as c_ulonglong,
            LLVMBool::from(false),
        );

        let result_type_name = CString::new("res_type").unwrap();
        let result_type_t = LLVMInt8TypeInContext(context);
        // Create stack space for result type (i8)
        let res_type_alloc =
            LLVMBuildAlloca(self.builder, result_type_t, result_type_name.as_ptr());
        // TODO: enable pointer results
        let opaque_ptr_t = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
        let result_alloc_i8 =
            LLVMBuildBitCast(self.builder, result_alloc, opaque_ptr_t, EMPTY_STR.as_ptr());

        let mut args = [
            self.runtime_ref,
            name_value,
            stack_size_value,
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
        res_type_alloc
    }

    fn push_args(&self, args: &[Alloc]) {
        let context = unsafe { LLVMGetModuleContext(self.module) };
        args.iter()
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
                let value_discriminator = self.expr_builder.build_value(&discriminator);
                (value_discriminator, alloc)
            })
            .map(|(disc, value)| unsafe {
                let opaque_ptr_t = LLVMPointerType(LLVMInt8TypeInContext(context), 0);
                let casted =
                    LLVMBuildBitCast(self.builder, *value, opaque_ptr_t, EMPTY_STR.as_ptr());
                (disc, casted)
            })
            .for_each(|(disc, arg_ptr)| {
                self.procedure_generic_push_arg(disc, arg_ptr);
            });
    }
}
