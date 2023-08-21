use crate::ast::Expr;

type AllocId = usize;

#[derive(Debug)]
pub enum CompilispIr {
    AllocVar(AllocId),
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
        args: Vec<AllocId>,
    },
    ProcedureScopeStart,
    ProcedureScopeEnd,
    ProcedureReturnValue(AllocId),
    PushArg(AllocId),
}

// Todo: generate ir in a lazy way
#[derive(Debug)]
pub struct CompilispIrGenerator {
    ir_buffer: Vec<CompilispIr>,
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

    fn process_expr(&mut self, expr: &Expr) -> AllocId {
        match expr {
            Expr::Number(value) => {
                self.alloc_id += 1;
                self.ir_buffer.push(CompilispIr::AllocVar(self.alloc_id));
                self.ir_buffer.push(CompilispIr::ConstInt {
                    alloc_id: self.alloc_id,
                    value: *value,
                });
                self.alloc_id
            }

            Expr::String(value) => {
                self.alloc_id += 1;
                self.ir_buffer.push(CompilispIr::AllocVar(self.alloc_id));
                self.ir_buffer.push(CompilispIr::GlobalString {
                    alloc_id: self.alloc_id,
                    value: value.clone(),
                });
                self.alloc_id
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
                return_alloc_id
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
