# Wickra Backtest — C / C++ examples

The Wickra Backtest C ABI is a single shared/static library plus a generated header
([`bindings/c/include/wickra_backtest.h`](../../bindings/c/include/wickra_backtest.h)). Any C-capable
language links against the same artifact; these examples show the plain-C path
and, through [`wickra_backtest.hpp`](../../bindings/c/include/wickra_backtest.hpp), the C++ one.

## Build the library

From the workspace root:

```sh
cargo build -p wickra-backtest-c --release
```

This produces, in `target/release/`:

| Platform | Shared library | Link target |
|----------|----------------|-------------|
| Linux    | `libwickra_backtest.so`     | `-lwickra_backtest` |
| macOS    | `libwickra_backtest.dylib`  | `-lwickra_backtest` |
| Windows (MSVC) | `wickra_backtest.dll` | `wickra_backtest.dll.lib` (import lib) |

A static library (`libwickra_backtest.a` / `wickra_backtest.lib`) is emitted alongside.

## Build and run the examples

With CMake, as the CI C ABI job does:

```sh
cmake -S examples/c -B examples/c/build
cmake --build examples/c/build --config Release
ctest --test-dir examples/c/build -C Release --output-on-failure
```

## The examples

| Example | What it does |
|---------|--------------|
| `cpp_smoke.cpp` | C++ example for the wickra-backtest C ABI, through the optional RAII wrapper (`wickra_backtest.hpp`). |
| `example.c` | Minimal C example for the wickra-backtest C ABI. |
| `example_cpp.cpp` | C++ build of the minimal C example. |
| `streaming.c` | Streaming example for the wickra-backtest C ABI. |
| `streaming_cpp.cpp` | C++ build of the streaming example, from the same source as the C target. |

## Usage shape

Every call follows the same handle discipline: construct from a spec JSON, drive
with command JSON, read the response, free the handle exactly once. `wickra_backtest.h` is
the whole contract; the C++ header, where one ships, wraps the handle in a
move-only RAII type. See [`bindings/c/README.md`](../../bindings/c/README.md).
