TESTS=let_sum conditional let_nested
LLVM_FILES=$(patsubst %, %.ll, $(TESTS))

all: $(TESTS)

test:
	pytest tests/execute.py

%.ll: tests/%.scheme 
	cargo run -- $<

%: %.ll
	clang tests/$^ target/debug/libruntime.a -o $@

clean:
	$(RM) $(TESTS) $(LLVM_FILES)

.PHONY: all clean
