use crate::runtime::{CompilispObject, CompilispRuntime, CompilispValue};
use std::ffi::{c_char, CStr};
use std::io;
use std::io::Write;
use std::slice::from_raw_parts;

#[no_mangle]
pub extern "C" fn compilisp_init() -> *mut CompilispRuntime {
    let b = Box::new(CompilispRuntime::default());
    Box::into_raw(b)
}

/// # Safety
/// _self must be a valid pointer to a compilisp runtime
#[no_mangle]
pub unsafe extern "C" fn compilisp_destroy(_self: *mut CompilispRuntime) {
    io::stdout().flush().ok();
    drop(Box::from_raw(_self));
}

#[no_mangle]
pub unsafe extern "C" fn compilisp_procedure_call(
    name: *const c_char,
    argv: *const CompilispObject,
    argc: u32,
) -> CompilispObject {
    let name = CStr::from_ptr(name);
    let objects = from_raw_parts(argv, argc as usize);
    let args = objects
        .iter()
        .flat_map(CompilispValue::try_from)
        .collect::<Vec<_>>();
    let procedure_name = name.to_str().unwrap();
    let result = CompilispRuntime::procedure_call(procedure_name, args.as_slice()).unwrap();
    CompilispObject::from(&result)
}
