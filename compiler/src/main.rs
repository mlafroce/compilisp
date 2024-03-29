#[macro_use]
extern crate lalrpop_util;

use clap::Parser;
use compilisp::ast::ModuleAst;
use compilisp::backend::llvm_context::Context;
use lalrpop_util::ParseError;
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

    let mut errors = Vec::new();
    let parser = lisp::ModuleParser::new();
    match parser.parse(&mut errors, &module_text) {
        Ok(expr_vec) => {
            if !errors.is_empty() {
                println!("Compilation aborted");
                for error in errors {
                    match error.error {
                        ParseError::UnrecognizedToken { token, expected } => {
                            println!(
                                "Unexepected token: {:?}, expecting: {:?}",
                                token.1 .1, expected
                            )
                        }
                        _ => {
                            println!("Parser error: {:?}", error)
                        }
                    }
                }
            } else {
                let compiler = Context::new();
                let source = args.input.to_string_lossy().to_string();
                let root = ModuleAst { expr_vec, source };
                compiler.add_module(root);
            }
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
