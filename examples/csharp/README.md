# Wickra Backtest examples — C#

Runnable C# examples for the [Wickra Backtest C# binding](../../bindings/csharp). The binding consumes the C ABI
library through P/Invoke, so build it once before running anything:

```bash
cargo build -p wickra-backtest-c --release
```

## Run

As the CI examples job runs it, from the repository root:

```bash
dotnet run --project examples/csharp/<Example>
```

## The examples

| Example | What it does |
|---------|--------------|
| `Program.cs` | Run the shared EMA-cross strategy from C#, both ways. |
