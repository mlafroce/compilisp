#[derive(Clone, Debug)]
pub enum Expr {
    Number(i32),
    Boolean(bool),
    Symbol(String),
    String(String),
    List(Vec<Expr>),
    Procedure(String, Vec<Expr>),
    LetProcedure(Vec<(String, Expr)>, Box<Expr>),
}

#[derive(Debug)]
pub struct ModuleAst {
    pub source: String,
    pub root: Expr,
}
