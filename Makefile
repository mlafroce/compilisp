all: let_sum

let_sum.ll: let_sum.scheme 
	cargo run -- let_sum.scheme
let_sum: let_sum.ll
	clang $^ target/debug/libruntime.a -o $@

.PHONY: let_sum
