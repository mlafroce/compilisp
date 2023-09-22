#include "llvm-c/Target.h"
#include "llvm/IR/DataLayout.h"
#include "llvm/IR/Module.h"

extern "C" unsigned int LLVMCompilispGetTypeAllocSize(LLVMTargetDataRef D, LLVMTypeRef Ty) {
    return llvm::unwrap(D)->getTypeAllocSize(llvm::unwrap(Ty));
}