cmake_minimum_required(VERSION 2.9)

project(llvm-compilisp)

# Based on https://llvm.org/docs/CMake.html

find_package(LLVM REQUIRED CONFIG)

add_library(llvm-compilisp
    llvm-compilisp.cpp
)

include_directories(${LLVM_INCLUDE_DIRS})
separate_arguments(LLVM_DEFINITIONS_LIST NATIVE_COMMAND ${LLVM_DEFINITIONS})
add_definitions(${LLVM_DEFINITIONS_LIST})

# Find the libraries that correspond to the LLVM components
# that we wish to use
llvm_map_components_to_libnames(llvm_libs support core irreader)

# Link against LLVM libraries
target_link_libraries(llvm-compilisp ${llvm_libs})

install(TARGETS llvm-compilisp DESTINATION .)
