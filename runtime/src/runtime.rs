use crate::runtime::CompilispValue::Number;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};

#[derive(Debug)]
pub enum CompilispError {
    UnboundVariable(String),
    ArgTypeMismatch,
}

type CompilispResult<T> = Result<T, CompilispError>;

#[derive(Debug)]
pub enum CompilispValue {
    Number(i32),
    String(String),
}

#[derive(Default)]
pub struct CompilispRuntime {
    scopes: Vec<HashMap<String, CompilispValue>>,
    args: Vec<CompilispValue>,
}

impl CompilispRuntime {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn push_let_context(&mut self) {
        println!("-> Pushing let context");
        self.scopes.push(HashMap::new());
    }

    pub fn push_let_binding(&mut self, bind_name: &str, bind_value: CompilispValue) {
        println!("-- Binding {:?} -> {:?}", bind_name, bind_value);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(bind_name.to_owned(), bind_value);
        }
    }

    pub fn pop_let_context(&mut self) {
        println!("<- Pop let context");
        self.scopes.pop();
    }

    pub fn procedure_push_arg(&mut self, arg: CompilispValue) {
        println!("-> Push process arg: {:?}", arg);
        self.args.push(arg);
    }

    pub fn procedure_call(&mut self, procedure_name: &str) -> CompilispResult<CompilispValue> {
        println!("-- procedure call: {:?}", procedure_name);
        match procedure_name {
            "+" => {
                let result = compilisp_sum(self.args.as_slice());
                self.args.clear();
                result
            }
            "display" => {
                if let Some(value) = self.args.get(0) {
                    println!("Display: {:?}", value);
                } else {
                    println!("Display: Nul");
                }
                self.args.clear();
                // TODO: void return
                Ok(Number(0))
            }
            _ => {
                self.args.clear();
                Err(CompilispError::UnboundVariable(procedure_name.to_string()))
            }
        }
    }
}

fn compilisp_sum(args: &[CompilispValue]) -> CompilispResult<CompilispValue> {
    let mut result = 0;
    for arg in args {
        if let CompilispValue::Number(value) = arg {
            result += value;
        } else {
            return Err(CompilispError::ArgTypeMismatch);
        }
    }
    Ok(CompilispValue::Number(result))
}

#[no_mangle]
pub extern "C" fn compilisp_init() -> *mut CompilispRuntime {
    println!("Compilisp init called");
    let b = Box::new(CompilispRuntime::new());
    Box::into_raw(b)
}

/// # Safety
/// _self must be a valid pointer to a compilisp runtime
#[no_mangle]
pub unsafe extern "C" fn compilisp_destroy(_self: *mut CompilispRuntime) {
    println!("Compilisp destroy called");
    drop(Box::from_raw(_self));
}

/// # Safety
/// _self must be a valid pointer to a compilisp runtime
#[no_mangle]
pub unsafe extern "C" fn compilisp_procedure_push_arg(
    _self: *mut CompilispRuntime,
    bind_type: u8,
    bind_value: *const c_void,
) {
    let mut _self = &mut *_self;
    let arg = opaque_to_enum(bind_type, bind_value);
    _self.procedure_push_arg(arg);
}

/// # Safety
/// _self must be a valid pointer to a compilisp runtime
#[no_mangle]
pub unsafe extern "C" fn compilisp_procedure_call(
    _self: *mut CompilispRuntime,
    procedure_name: *const c_char,
    result_type: *mut i8,
    result_value: *mut i32,
) -> i32 {
    let mut _self = &mut *_self;
    let c_procedure_name = CStr::from_ptr(procedure_name);
    let result = _self.procedure_call(c_procedure_name.to_str().unwrap());
    println!("Should return {:?}", result);
    if let Ok(value) = result {
        match value {
            CompilispValue::Number(value) => {
                *result_type = 0;
                *result_value = value;
            }
            _ => panic!("Only number operations supported"),
        }
        return 0;
    } else {
        match result {
            Err(CompilispError::UnboundVariable(_)) => return 1,
            Err(CompilispError::ArgTypeMismatch) => return 2,
            _ => unreachable!(),
        }
    }
}

/// # Safety
/// _self must be a valid pointer to a compilisp runtime
#[no_mangle]
pub unsafe extern "C" fn compilisp_push_let_context(_self: *mut CompilispRuntime) {
    let mut _self = &mut *_self;
    _self.push_let_context();
}

/// # Safety
/// _self must be a valid pointer to a compilisp runtime
#[no_mangle]
pub unsafe extern "C" fn compilisp_push_let_binding(
    _self: *mut CompilispRuntime,
    bind_name: *const c_char,
    bind_type: u8,
    bind_value: *const c_void,
) {
    let mut _self = &mut *_self;
    let bind_name = CStr::from_ptr(bind_name).to_str().unwrap();
    let bind_value = opaque_to_enum(bind_type, bind_value);
    _self.push_let_binding(bind_name, bind_value);
}

#[no_mangle]
pub unsafe extern "C" fn compilisp_pop_let_context(_self: *mut CompilispRuntime) {
    let mut _self = &mut *_self;
    _self.pop_let_context();
}

#[no_mangle]
pub unsafe extern "C" fn compilisp_push_arg(_self: *mut CompilispRuntime) {
    let mut _self = &mut *_self;
    _self.pop_let_context();
}

unsafe fn opaque_to_enum(bind_type: u8, bind_value: *const c_void) -> CompilispValue {
    if bind_type == 0 {
        let value = *(bind_value as *const i32);
        CompilispValue::Number(value)
    } else {
        let value = CStr::from_ptr(bind_value as *const c_char)
            .to_str()
            .unwrap();
        CompilispValue::String(value.to_owned())
    }
}
