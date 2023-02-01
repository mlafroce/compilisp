use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::io;
use std::io::Write;

#[derive(Debug)]
pub enum CompilispError {
    UnboundVariable(String),
    ArgTypeMismatch,
}

type CompilispResult<T> = Result<T, CompilispError>;

#[derive(Debug)]
pub enum CompilispValue {
    Number(i32),
    Boolean(bool),
    String(String),
    Symbol(String),
}

#[derive(Default)]
pub struct CompilispRuntime {
    scopes: Vec<HashMap<String, CompilispValue>>,
    args: Vec<CompilispValue>,
}

impl<'a> CompilispRuntime {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn push_let_context(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn push_let_binding(&mut self, bind_name: &str, bind_value: CompilispValue) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(bind_name.to_owned(), bind_value);
        }
    }

    pub fn pop_let_context(&mut self) {
        self.scopes.pop();
    }

    pub fn procedure_push_arg(&mut self, arg: CompilispValue) {
        self.args.push(arg);
    }

    pub fn procedure_call(
        &mut self,
        procedure_name: &str,
        stack_size: u8,
    ) -> CompilispResult<CompilispValue> {
        let args = self.pop_args(stack_size);
        match procedure_name {
            "+" => {
                let resolved_args = self.resolve(&args)?;
                let result = compilisp_sum(resolved_args.as_slice());
                result
            }
            "<" => {
                let resolved_args = self.resolve(&args)?;
                let result = compilisp_le(resolved_args.as_slice());
                result
            }
            "display" => {
                for value in &args {
                    match value {
                        CompilispValue::Number(num) => print!("{num}"),
                        CompilispValue::Boolean(num) => print!("{num}"),
                        CompilispValue::String(value) => print!("{value}"),
                        CompilispValue::Symbol(_) => {
                            if let Ok(value) = self.resolve_symbol(value) {
                                print!("{value:?}");
                            } else {
                                print!("Nil");
                            }
                        }
                    }
                    print!("");
                }
                // TODO: void return
                Ok(CompilispValue::Number(0))
            }
            _ => Err(CompilispError::UnboundVariable(procedure_name.to_string())),
        }
    }

    fn pop_args(&mut self, stack_size: u8) -> Vec<CompilispValue> {
        let new_len = self.args.len() - stack_size as usize;
        self.args.drain(new_len..).collect::<Vec<_>>()
    }

    fn resolve(&'a self, args: &'a [CompilispValue]) -> CompilispResult<Vec<&CompilispValue>> {
        args.iter()
            .map(|value| self.resolve_symbol(value))
            .collect()
    }

    fn resolve_symbol(&'a self, value: &'a CompilispValue) -> CompilispResult<&CompilispValue> {
        if let CompilispValue::Symbol(name) = value {
            let scope = self.scopes.last().unwrap();
            let resolved = scope.get(name);
            resolved.ok_or(CompilispError::UnboundVariable(name.clone()))
        } else {
            Ok(value)
        }
    }
}

fn compilisp_le(args: &[&CompilispValue]) -> CompilispResult<CompilispValue> {

    for slice in args.windows(2) {
        match (slice[0], slice[1]) {
            (CompilispValue::Number(lhs), CompilispValue::Number(rhs)) => {
                if lhs >= rhs {
                    return Ok(CompilispValue::Boolean(false))
                }
            }
            _ => return Err(CompilispError::ArgTypeMismatch),
        }
    }
    Ok(CompilispValue::Boolean(true))
}

fn compilisp_sum(args: &[&CompilispValue]) -> CompilispResult<CompilispValue> {
    let mut result = 0;
    for arg in args {
        match arg {
            CompilispValue::Number(value) => {
                result += value;
            }
            _ => {
                return Err(CompilispError::ArgTypeMismatch);
            }
        }
    }
    Ok(CompilispValue::Number(result))
}

#[no_mangle]
pub extern "C" fn compilisp_init() -> *mut CompilispRuntime {
    let b = Box::new(CompilispRuntime::new());
    Box::into_raw(b)
}

/// # Safety
/// _self must be a valid pointer to a compilisp runtime
#[no_mangle]
pub unsafe extern "C" fn compilisp_destroy(_self: *mut CompilispRuntime) {
    io::stdout().flush().ok();
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
    stack_size: u8,
    result_type: *mut i8,
    result_value: *mut i32,
) -> i32 {
    let mut _self = &mut *_self;
    let c_procedure_name = CStr::from_ptr(procedure_name);
    let result = _self.procedure_call(c_procedure_name.to_str().unwrap(), stack_size);
    if let Ok(value) = result {
        match value {
            CompilispValue::Number(value) => {
                *result_type = 0;
                *result_value = value;
            }
            CompilispValue::Boolean(value) => {
                *result_type = 1;
                *result_value = if value { 1 } else { 0 };
            }
            _ => panic!("Only number operations supported"),
        }
        0
    } else {
        match result {
            Err(CompilispError::UnboundVariable(_)) => 1,
            Err(CompilispError::ArgTypeMismatch) => 2,
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

/// Pushes a new let binding scope
/// # Safety
/// _self is a valid CompilispRuntime instance
#[no_mangle]
pub unsafe extern "C" fn compilisp_pop_let_context(_self: *mut CompilispRuntime) {
    let mut _self = &mut *_self;
    _self.pop_let_context();
}

/// Removes last let binding scope
/// # Safety
/// _self is a valid CompilispRuntime instance
#[no_mangle]
pub unsafe extern "C" fn compilisp_push_arg(_self: *mut CompilispRuntime) {
    let mut _self = &mut *_self;
    _self.pop_let_context();
}

unsafe fn opaque_to_enum(bind_type: u8, bind_value: *const c_void) -> CompilispValue {
    match bind_type {
        0 => {
            let value = *(bind_value as *const i32);
            CompilispValue::Number(value)
        }
        1 => {
            let value = *(bind_value as *const i32);
            CompilispValue::Boolean(value != 0)
        }
        2 => {
            let value = CStr::from_ptr(bind_value as *const c_char)
                .to_str()
                .unwrap();
            CompilispValue::String(value.to_owned())
        }
        _ => {
            let value = CStr::from_ptr(bind_value as *const c_char)
                .to_str()
                .unwrap();
            CompilispValue::Symbol(value.to_owned())
        }
    }
}
