use crate::ast::Expr;
use std::collections::HashMap;

pub type AllocId = usize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AllocType {
    Int,
    String,
    Bool,
}

#[derive(Clone, Debug)]
pub struct Alloc {
    pub alloc_type: AllocType,
    pub id: AllocId,
}

#[derive(Clone, Debug)]
pub enum CompilispIr {
    CallProcedure {
        name: String,
        return_id: AllocId,
        args: Vec<Alloc>,
    },
    ConstInt {
        alloc_id: AllocId,
        value: i32,
    },
    GlobalString {
        alloc_id: AllocId,
        value: String,
    },
    IfExpressionEval {
        cond_alloc: AllocId,
    },
    IfExpressionElse {
        result_alloc: AllocId,
    },
    IfExpressionEndElse,
    IfExpressionEndThen,
    IfExpressionEndBlock {
        result_alloc: AllocId,
    },
    IfExpressionEndExpression {
        result_alloc: AllocId,
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
    symbol_scopes: Vec<HashMap<String, Alloc>>,
    alloc_id: usize,
}

impl CompilispIrGenerator {
    pub fn new() -> Self {
        let ir_buffer = vec![];
        let symbol_scopes = vec![HashMap::new()];
        Self {
            ir_buffer,
            alloc_id: 0,
            symbol_scopes,
        }
    }

    pub fn process(&mut self, root: Expr) {
        self.process_expr(&root);
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
            Expr::Procedure(name, args) => match name.as_str() {
                "if" => self.build_if_call(args.as_slice()),
                _ => self.build_generic_call(name, args),
            },
            Expr::LetProcedure(symbols, expr) => {
                self.push_let_context();
                for (symbol_name, sym_expr) in symbols {
                    let alloc = self.process_expr(sym_expr);
                    self.push_let_binding(symbol_name, alloc);
                }
                let result = self.process_expr(expr);
                self.pop_let_context();
                result
            }
            Expr::Symbol(name) => self
                .resolve_symbol(name)
                .expect("Symbol doesn't exist")
                .clone(),
            Expr::DefineExpr(name, value) => {
                // I'm not sure what to do with these
                let alloc = self.process_expr(value);
                self.push_let_binding(name, alloc.clone());
                alloc
            }
            _ => {
                unimplemented!("Cannot process this token yet {:?}", expr)
            }
        }
    }

    fn build_if_call(&mut self, args: &[Expr]) -> Alloc {
        let cond_expr = &args[0];
        // if(cond_expr)
        let cond_alloc = self.process_expr(cond_expr);
        self.ir_buffer.push(CompilispIr::IfExpressionEval {
            cond_alloc: cond_alloc.id,
        });
        // then {
        let then_expr = &args[1];
        let _ = self.process_expr(then_expr);
        self.ir_buffer.push(CompilispIr::IfExpressionEndThen);
        // } else {
        if let Some(else_expr) = args.get(2) {
            self.ir_buffer.push(CompilispIr::IfExpressionElse {
                result_alloc: cond_alloc.id,
            });
            let _ = self.process_expr(else_expr);
            self.ir_buffer.push(CompilispIr::IfExpressionEndElse);
        }
        // } finally
        self.ir_buffer.push(CompilispIr::IfExpressionEndBlock {
            result_alloc: cond_alloc.id,
        });

        cond_alloc
    }

    fn build_generic_call(&mut self, name: &str, args: &Vec<Expr>) -> Alloc {
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
        let name = name.to_owned();
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

    fn push_let_binding(&mut self, bind_name: &str, bind_value: Alloc) {
        if let Some(scope) = self.symbol_scopes.last_mut() {
            scope.insert(bind_name.to_owned(), bind_value);
        }
    }

    fn push_let_context(&mut self) {
        self.symbol_scopes.push(HashMap::new());
    }
    fn pop_let_context(&mut self) {
        self.symbol_scopes.pop();
    }

    fn resolve_symbol(&self, symbol_name: &str) -> Option<&Alloc> {
        self.symbol_scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.get(symbol_name))
            .next()
    }
}
