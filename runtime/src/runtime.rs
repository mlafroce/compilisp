use std::ffi::{c_char, CStr};
use std::fmt::Debug;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum CompilispType {
    Number,
    Boolean,
    String,
    Symbol,
}

#[repr(C)]
pub union CompilispObjectValue {
    int_value: i32,
    bool_value: bool,
    str_value: *mut c_char,
}

#[repr(C)]
pub struct CompilispObject {
    type_: CompilispType,
    value: CompilispObjectValue,
}

#[derive(Debug)]
pub enum CompilispError {
    UnboundVariable(String),
    ArgTypeMismatch,
}

type CompilispResult<T> = Result<T, CompilispError>;

#[derive(Clone, Debug)]
pub enum CompilispValue {
    Number(i32),
    Boolean(bool),
    String(String),
    Symbol(String),
}

#[derive(Default)]
pub struct CompilispRuntime;

impl CompilispRuntime {
    pub fn procedure_call(
        procedure_name: &str,
        args: &[CompilispValue],
    ) -> CompilispResult<CompilispValue> {
        match procedure_name {
            "+" => compilisp_sum(args),
            "<" => compilisp_le(args),
            "display" => {
                for value in args {
                    match value {
                        CompilispValue::Number(num) => print!("{num}"),
                        CompilispValue::Boolean(num) => print!("{num}"),
                        CompilispValue::String(value) => print!("{value}"),
                        CompilispValue::Symbol(_) => {
                            panic!("Unexepected value");
                        }
                    }
                }
                // TODO: void return
                Ok(CompilispValue::Number(0))
            }
            "begin" => args.last().cloned().ok_or(CompilispError::ArgTypeMismatch),
            _ => Err(CompilispError::UnboundVariable(procedure_name.to_string())),
        }
    }
}

fn compilisp_le(args: &[CompilispValue]) -> CompilispResult<CompilispValue> {
    for slice in args.windows(2) {
        match (&slice[0], &slice[1]) {
            (CompilispValue::Number(lhs), CompilispValue::Number(rhs)) => {
                if lhs >= rhs {
                    return Ok(CompilispValue::Boolean(false));
                }
            }
            _ => return Err(CompilispError::ArgTypeMismatch),
        }
    }
    Ok(CompilispValue::Boolean(true))
}

fn compilisp_sum(args: &[CompilispValue]) -> CompilispResult<CompilispValue> {
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

impl TryFrom<&CompilispObject> for CompilispValue {
    type Error = ();

    fn try_from(obj: &CompilispObject) -> Result<Self, Self::Error> {
        match obj.type_ {
            CompilispType::Number => Ok(CompilispValue::Number(unsafe { obj.value.int_value })),
            CompilispType::Boolean => Ok(CompilispValue::Boolean(unsafe { obj.value.bool_value })),
            CompilispType::String => unsafe {
                let s = CStr::from_ptr(obj.value.str_value);
                Ok(CompilispValue::String(s.to_str().unwrap().to_string()))
            },
            _ => Err(()),
        }
    }
}

impl From<&CompilispValue> for CompilispObject {
    fn from(value: &CompilispValue) -> Self {
        match value {
            CompilispValue::Number(value) => {
                let value = CompilispObjectValue { int_value: *value };
                Self {
                    type_: CompilispType::Number,
                    value,
                }
            }
            CompilispValue::Boolean(value) => {
                let value = CompilispObjectValue { bool_value: *value };
                Self {
                    type_: CompilispType::Boolean,
                    value,
                }
            }
            _ => todo!("Unsupported return value"),
        }
    }
}
