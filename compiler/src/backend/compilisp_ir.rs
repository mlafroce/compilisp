use crate::ast::Expr;

pub type AllocId = usize;

#[derive(Clone, Copy, Debug)]
pub enum AllocType {
    Int,
    String,
    Bool,
}

#[derive(Debug)]
pub struct Alloc {
    pub alloc_type: AllocType,
    pub id: AllocId,
}

#[derive(Debug)]
pub enum CompilispIr {
    ConstInt {
        alloc_id: AllocId,
        value: i32,
    },
    GlobalString {
        alloc_id: AllocId,
        value: String,
    },
    CallProcedure {
        name: String,
        return_id: AllocId,
        args: Vec<Alloc>,
    },
    ProcedureScopeStart,
    ProcedureScopeEnd,
    ProcedureReturnValue(AllocId),
    PushArg(AllocId),
}

// Todo: generate ir in a lazy way and make buffer private
#[derive(Debug)]
pub struct CompilispIrGenerator {
    pub ir_buffer: Vec<CompilispIr>,
    alloc_id: usize,
}

impl CompilispIrGenerator {
    pub fn new(root: &Expr) -> Self {
        let ir_buffer = vec![];
        let mut ret = Self {
            ir_buffer,
            alloc_id: 0,
        };
        ret.process_expr(root);
        ret
    }

    fn process_expr(&mut self, expr: &Expr) -> Alloc {
        match expr {
            Expr::Number(value) => {
                self.alloc_id += 1;
                self.ir_buffer.push(CompilispIr::ConstInt {
                    alloc_id: self.alloc_id,
                    value: *value,
                });
                Alloc {
                    id: self.alloc_id,
                    alloc_type: AllocType::Int,
                }
            }

            Expr::String(value) => {
                self.alloc_id += 1;
                self.ir_buffer.push(CompilispIr::GlobalString {
                    alloc_id: self.alloc_id,
                    value: value.clone(),
                });
                Alloc {
                    id: self.alloc_id,
                    alloc_type: AllocType::String,
                }
            }
            Expr::Procedure(name, args) => {
                self.alloc_id += 1;
                let return_alloc_id = self.alloc_id;
                let mut call_args = vec![];
                self.ir_buffer
                    .push(CompilispIr::ProcedureReturnValue(return_alloc_id));
                self.ir_buffer.push(CompilispIr::ProcedureScopeStart);
                for arg in args {
                    let alloc_id = self.process_expr(arg);
                    call_args.push(alloc_id);
                }
                let name = name.clone();
                self.ir_buffer.push(CompilispIr::CallProcedure {
                    name,
                    return_id: return_alloc_id,
                    args: call_args,
                });
                self.ir_buffer.push(CompilispIr::ProcedureScopeEnd);
                Alloc {
                    id: return_alloc_id,
                    alloc_type: AllocType::Int,
                }
            }
            Expr::LetProcedure(symbols, expr) => {
                for (_symbol_name, sym_expr) in symbols {
                    let _ = self.process_expr(sym_expr);
                }
                self.process_expr(expr)
            }
            _ => {
                unimplemented!("Cannot process this token yet {:?}", expr)
            }
        }
    }
}
