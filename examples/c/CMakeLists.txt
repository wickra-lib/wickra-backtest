cmake_minimum_required(VERSION 3.15)
project(wickra_backtest_c_examples C)

# Directory holding the compiled wickra-backtest C library (cargo output), e.g.
# <workspace>/target/release. Override with -DWICKRA_LIB_DIR=/path/to/target/release.
if(NOT DEFINED WICKRA_LIB_DIR)
    set(WICKRA_LIB_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release")
endif()
set(WICKRA_INCLUDE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../bindings/c/include")

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

# The cross-language smoke test: a foreign C consumer links the generated header
# and the compiled library, then runs. On Windows the DLL is copied next to the
# executable so the loader finds it; on Unix LD_LIBRARY_PATH / DYLD_LIBRARY_PATH
# point at the cargo output directory.
add_executable(example example.c)
target_include_directories(example PRIVATE "${WICKRA_INCLUDE_DIR}")
target_link_libraries(example PRIVATE "${WICKRA_LINK_LIB}")
if(UNIX AND NOT APPLE)
    target_link_libraries(example PRIVATE m)
endif()
if(WIN32)
    add_custom_command(TARGET example POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${WICKRA_RUNTIME}" "$<TARGET_FILE_DIR:example>")
endif()

add_test(NAME example COMMAND example)
if(NOT WIN32)
    set_tests_properties(example PROPERTIES
        ENVIRONMENT "LD_LIBRARY_PATH=${WICKRA_LIB_DIR};DYLD_LIBRARY_PATH=${WICKRA_LIB_DIR}")
endif()

# The streaming reach, driven from the same foreign C consumer. It runs the same
# bars through the handle and through the batch entry point and exits non-zero if
# the two reports differ, so "backtest and live are one code path" is checked from
# outside Rust rather than only asserted inside it.
add_executable(streaming streaming.c)
target_include_directories(streaming PRIVATE "${WICKRA_INCLUDE_DIR}")
target_link_libraries(streaming PRIVATE "${WICKRA_LINK_LIB}")
if(UNIX AND NOT APPLE)
    target_link_libraries(streaming PRIVATE m)
endif()
if(WIN32)
    add_custom_command(TARGET streaming POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${WICKRA_RUNTIME}" "$<TARGET_FILE_DIR:streaming>")
endif()

add_test(NAME streaming COMMAND streaming)
if(NOT WIN32)
    set_tests_properties(streaming PROPERTIES
        ENVIRONMENT "LD_LIBRARY_PATH=${WICKRA_LIB_DIR};DYLD_LIBRARY_PATH=${WICKRA_LIB_DIR}")
endif()
