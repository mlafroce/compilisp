# Compilisp

Compiler of *Compilisp* language, a subset of Scheme interpreter. Compiles to LLVM IR.

## Workspace composition

* **Compiler**: Compilisp language compile. Uses `lalrpop` as frontend parser, and `llvm-sys` as backend compiler

* **Runtime**: The compiler calls some external functions defined in this runtime library. This runtime library helps to work with procedure calls and contexts.

## Usage

Compile the compiler with `cargo build`

Call with `cargo run -- <input_file.scheme>`. This will generate a `<input_file.ll>` file.

Compile with clang, link against runtime.

Use Makefile as an example
