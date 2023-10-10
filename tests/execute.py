import os
import pytest
import subprocess

COMPILISP_PATH = "target/debug/compilisp"
RUNTIME_PATH = "target/debug/libruntime.a"

subprocess.check_output(["cargo", "build"])

@pytest.fixture(scope='session')
def build_compiler():
    output = subprocess.run(["cargo", "build"])
    assert(output.returncode == 0)


@pytest.mark.parametrize(
    'testcase',
    [
        "one_plus_two",
        "begin_01",
        "let_sum",
        "let_nested",
        "conditional",
        "conditional_many",
        "conditional_display",
        "define_expr_01",
        "define_procedure_01",
        "define_procedure_02"
    ]
)
def test_compile_and_run(testcase):
    filepath = "tests/" + testcase + ".scheme"
    ir_path = "tests/" + testcase + ".ll"
    executable_path = "./" + testcase
    with open(filepath) as input:
        scheme_output = execute_scheme(input)
    sp_output = execute_compilisp(filepath)
    assert(sp_output.returncode == 0)
    clang_output = execute_clang(ir_path, executable_path)
    assert(clang_output.returncode == 0)
    compilisp_output = execute_compiled(executable_path)
    assert(scheme_output == compilisp_output)
    os.remove(executable_path)

def execute_scheme(input):
    output = subprocess.check_output(["scheme", "--quiet"], stdin=input)
    return output

def execute_compilisp(input_path):
    output = subprocess.run([COMPILISP_PATH, input_path])
    return output

def execute_clang(ir_path, executable_path):
    output = subprocess.run(["clang", ir_path, RUNTIME_PATH, "-o", executable_path])
    return output

def execute_compiled(executable):
    output = subprocess.check_output(executable)
    return output
