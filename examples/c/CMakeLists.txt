cmake_minimum_required(VERSION 3.15)
# C and C++ both, because both are advertised: the C ABI header is `extern "C"`
# and there is a header-only C++ wrapper beside it, so the C++ reach is compiled
# here rather than merely claimed in a README.
project(wickra_backtest_c_examples C CXX)

# Directory holding the compiled wickra-backtest C library (cargo output), e.g.
# <workspace>/target/release. Override with -DWICKRA_LIB_DIR=/path/to/target/release.
if(NOT DEFINED WICKRA_LIB_DIR)
    set(WICKRA_LIB_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release")
endif()
set(WICKRA_INCLUDE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../bindings/c/include")

# The wrapper uses std::exchange, so C++14 is the floor; ask for it rather than
# inheriting whatever the toolchain happens to default to.
set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Pick the right link target per platform/toolchain.
#  - MSVC links the generated import library (wickra_backtest.dll.lib).
#  - MinGW/gcc on Windows links the DLL directly.
#  - Unix links the shared object / dylib.
if(WIN32)
    set(WICKRA_RUNTIME "${WICKRA_LIB_DIR}/wickra_backtest.dll")
    if(MSVC)
        set(WICKRA_LINK_LIB "${WICKRA_LIB_DIR}/wickra_backtest.dll.lib")
    else()
        set(WICKRA_LINK_LIB "${WICKRA_LIB_DIR}/wickra_backtest.dll")
    endif()
elseif(APPLE)
    set(WICKRA_LINK_LIB "${WICKRA_LIB_DIR}/libwickra_backtest.dylib")
else()
    set(WICKRA_LINK_LIB "${WICKRA_LIB_DIR}/libwickra_backtest.so")
endif()

enable_testing()

# Build one example, link it to the compiled library, and register it as a
# ctest. On Windows the DLL is copied next to the executable so the loader finds
# it; on Unix the test's environment points at the cargo output directory.
function(add_wickra_example name source)
    add_executable(${name} ${source})
    target_include_directories(${name} PRIVATE "${WICKRA_INCLUDE_DIR}")
    target_link_libraries(${name} PRIVATE "${WICKRA_LINK_LIB}")
    if(UNIX AND NOT APPLE)
        target_link_libraries(${name} PRIVATE m)
    endif()
    if(WIN32)
        add_custom_command(TARGET ${name} POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "${WICKRA_RUNTIME}" "$<TARGET_FILE_DIR:${name}>")
    endif()
    add_test(NAME ${name} COMMAND ${name})
    if(NOT WIN32)
        set_tests_properties(${name} PROPERTIES
            ENVIRONMENT "LD_LIBRARY_PATH=${WICKRA_LIB_DIR};DYLD_LIBRARY_PATH=${WICKRA_LIB_DIR}")
    endif()
endfunction()

# The real cross-language test: a foreign consumer links the generated header and
# the compiled library, then runs. The streaming targets go further and compare
# the streamed report against the batch one, so "backtest and live are one code
# path" is checked from outside Rust rather than only asserted inside it.
add_wickra_example(example example.c)                  # batch run over a series
add_wickra_example(streaming streaming.c)              # streamed == batch, from C
add_wickra_example(example_cpp example_cpp.cpp)        # the same C source, as C++
add_wickra_example(streaming_cpp streaming_cpp.cpp)    # ditto, raw handles and frees
add_wickra_example(cpp_smoke cpp_smoke.cpp)            # C++ RAII wrapper (wickra_backtest.hpp)
