use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};

#[derive(Debug)]
pub enum CompilispValue {
    Number(i32),
    String(String),
}

#[derive(Default)]
pub struct CompilispRuntime {
    scopes: Vec<HashMap<String, CompilispValue>>
}

impl CompilispRuntime {
    pub fn new() -> Self {
        Self {.. Default::default()}
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
    if bind_type == 0 {
        let value = *(bind_value as *const i32);
        _self.push_let_binding(bind_name, CompilispValue::Number(value));
    } else {
        let value = CStr::from_ptr(bind_value as *const c_char)
            .to_str()
            .unwrap();
        _self.push_let_binding(bind_name, CompilispValue::String(value.to_owned()));
    }
}

#[no_mangle]
pub unsafe extern "C" fn compilisp_pop_let_context(_self: *mut CompilispRuntime) {
    let mut _self = &mut *_self;
    _self.pop_let_context();
}
