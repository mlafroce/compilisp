extern crate cmake;
extern crate lalrpop;

fn main() {
    let dst = cmake::build("llvmCompilisp");

    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=llvm-compilisp");
    lalrpop::process_root().unwrap();
}
