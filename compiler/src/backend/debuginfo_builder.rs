use llvm_sys::core::*;
use llvm_sys::LLVMModuleFlagBehavior;
use llvm_sys::debuginfo::LLVMDWARFEmissionKind::LLVMDWARFEmissionKindFull;
use llvm_sys::debuginfo::{
    LLVMCreateDIBuilder, LLVMDIBuilderCreateCompileUnit, LLVMDIBuilderCreateFile,
    LLVMDIBuilderFinalize, LLVMDWARFSourceLanguage, LLVMDisposeDIBuilder,
};
use llvm_sys::prelude::*;
use std::ffi::CString;

pub struct DebugInfoBuilder {
    builder_ref: LLVMDIBuilderRef
}

impl DebugInfoBuilder {
    pub unsafe fn new(module: LLVMModuleRef, name: &str) -> Self {
        let debug_version = CString::new("Debug Info Version").unwrap();
        let debug_version_value =
            LLVMValueAsMetadata(LLVMConstInt(LLVMInt32Type(), 3, LLVMBool::from(false)));
        LLVMAddModuleFlag(
            module,
            LLVMModuleFlagBehavior::LLVMModuleFlagBehaviorWarning,
            debug_version.as_ptr(),
            debug_version.as_bytes().len(),
            debug_version_value,
        );
        let name_in_debuginfo = CString::new(name).unwrap();
        let work_dir = CString::new(".").unwrap();
        let di_producer = CString::new("LLVM Compilisp").unwrap();
        let split_name = CString::new("Unknown data").unwrap();
        let flags = "\0";

        let builder_ref = LLVMCreateDIBuilder(module);
        let compile_unit_file = LLVMDIBuilderCreateFile(
            builder_ref,
            name_in_debuginfo.as_ptr(),
            name_in_debuginfo.as_bytes().len(),
            work_dir.as_ptr().cast(),
            work_dir.as_bytes().len(),
        );
        LLVMDIBuilderCreateCompileUnit(
            builder_ref,
            LLVMDWARFSourceLanguage::LLVMDWARFSourceLanguageC, // Should be Compilisp :P
            compile_unit_file,
            di_producer.as_ptr(),
            di_producer.as_bytes().len(),
            LLVMBool::from(false),
            flags.as_ptr().cast(),
            0,
            0,
            split_name.as_ptr(),
            split_name.as_bytes().len(),
            LLVMDWARFEmissionKindFull,
            1,
            LLVMBool::from(false),
            LLVMBool::from(false),
    //        "".as_ptr().cast(),
    //        0,
    //        "".as_ptr().cast(),
    //        0,
        );
        DebugInfoBuilder { builder_ref }
    }

    pub fn finalize(self) {
        unsafe { LLVMDIBuilderFinalize(self.builder_ref) };
    }
}

impl Drop for DebugInfoBuilder {
    fn drop(&mut self) {
        unsafe { LLVMDisposeDIBuilder(self.builder_ref) };
    }
}
