cmake_minimum_required(VERSION 3.15)
project(wickra_backtest_c_benchmarks C)

# Directory holding the compiled wickra-backtest C library (cargo output).
# Override with -DWICKRA_LIB_DIR=/path/to/target/release.
if(NOT DEFINED WICKRA_LIB_DIR)
    set(WICKRA_LIB_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release")
endif()
set(WICKRA_INCLUDE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../include")

# Same per-platform link rules as examples/c: MSVC takes the import library,
# MinGW links the DLL by name, Unix links the shared object.
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

add_executable(throughput throughput.c)
target_include_directories(throughput PRIVATE "${WICKRA_INCLUDE_DIR}")
target_link_libraries(throughput PRIVATE "${WICKRA_LINK_LIB}")
if(UNIX AND NOT APPLE)
    target_link_libraries(throughput PRIVATE m)
endif()
if(WIN32)
    add_custom_command(TARGET throughput POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${WICKRA_RUNTIME}" "$<TARGET_FILE_DIR:throughput>")
endif()
