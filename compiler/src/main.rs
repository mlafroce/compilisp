#[macro_use]
extern crate lalrpop_util;

use clap::Parser;
use compilisp::ast::ModuleAst;
use compilisp::backend::llvm::Context;
use std::fs::File;
use std::io;
use std::io::Read;

lalrpop_mod!(pub lisp); // synthesized by LALRPOP

#[derive(Parser)]
struct CliArgs {
    /// The pattern to look for
    input: std::path::PathBuf,
    /// The path to the file to read
    output: Option<std::path::PathBuf>,
}

fn main() {
    let args = CliArgs::parse();
    compile(args).unwrap();
}

fn compile(args: CliArgs) -> io::Result<()> {
    let mut module_file = File::open(&args.input)?;
    let mut module_text = String::new();
    module_file.read_to_string(&mut module_text)?;

    let parser = lisp::ExpressionParser::new();
    match parser.parse(&module_text) {
        Ok(root_expr) => {
            //println!("{:?}: {:?}", args.input, root_expr);
            let compiler = Context::new();
            let source = args.input.to_string_lossy().to_string();
            let root = ModuleAst {root: root_expr, source};
            compiler.add_module(root);
        }
        Err(e) => {
            println!("Failed to compile: {e:?}");
        }
    }
    Ok(())
}

#[test]
fn parse_sum() {
    let parser = lisp::ExpressionParser::new();
    let ast = parser.parse("(sum 2 3)");
    if let Ok(Expr::Procedure(sum, values)) = ast {
        assert_eq!(sum, "sum");
        assert_eq!(values.len(), 2);
    }
}

#[test]
fn parse_let() {
    let parser = lisp::ExpressionParser::new();
    let ast = parser.parse("(let ((x 2)) (+ 3 x))");
    println!("ast: {:?}", ast);
    assert!(ast.is_ok());
}
