use llvm_sys::core::{LLVMGetModuleContext, LLVMInt1TypeInContext, LLVMInt32TypeInContext, LLVMInt8TypeInContext, LLVMPointerType, LLVMStructCreateNamed, LLVMStructSetBody};
use llvm_sys::prelude::{LLVMBool, LLVMModuleRef, LLVMTypeRef};
use std::collections::HashMap;
use std::ffi::CString;

#[derive(Eq, Hash, PartialEq)]
pub enum CompilispType {
    Char,
    CharPtr,
    Int,
    IntPtr,
    BoolPtr,
    CompilispObject,
}

pub struct TypeFactory {
    type_map: HashMap<CompilispType, LLVMTypeRef>,
}

impl TypeFactory {
    pub fn new(module: LLVMModuleRef) -> Self {
        let mut type_map = HashMap::new();
        unsafe {
            let context = LLVMGetModuleContext(module);
            let char_type = LLVMInt8TypeInContext(context);
            type_map.insert(CompilispType::Char, char_type);

            let char_pointer = LLVMPointerType(char_type, 0);
            type_map.insert(CompilispType::CharPtr, char_pointer);

            let int_type = LLVMInt32TypeInContext(context);
            type_map.insert(CompilispType::Int, int_type);

            let int_pointer = LLVMPointerType(int_type, 0);
            type_map.insert(CompilispType::IntPtr, int_pointer);

            let bool_type = LLVMInt1TypeInContext(context);
            let bool_pointer = LLVMPointerType(bool_type, 0);
            type_map.insert(CompilispType::BoolPtr, bool_pointer);

            let struct_name = CString::new("compilisp_object").unwrap();
            let compilisp_object = LLVMStructCreateNamed(context, struct_name.as_ptr());
            // Biggest object data type is a pointer
            // Int objects are either smaller or equal pointers
            let mut members = [int_type, char_pointer];
            LLVMStructSetBody(
                compilisp_object,
                members.as_mut_ptr(),
                2,
                LLVMBool::from(false),
            );
            type_map.insert(CompilispType::CompilispObject, compilisp_object);
        }
        Self { type_map }
    }

    pub fn get_type(&self, type_: CompilispType) -> LLVMTypeRef {
        self.type_map.get(&type_).unwrap().clone()
    }

    pub fn get_pointer(&self, type_: LLVMTypeRef) -> LLVMTypeRef {
        unsafe { LLVMPointerType(type_, 0) }
    }

}
