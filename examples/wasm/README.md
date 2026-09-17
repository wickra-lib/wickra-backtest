# Wickra Backtest WASM examples

Browser demos for the [Wickra Backtest WASM binding](../../bindings/wasm): an HTML page whose module script
loads the package the same way (`init()`, then construct), builds the same
object every other binding builds and prints the same output into the page, so
the pattern transfers one-to-one to your own page.

## Build

The WASM module ships as a `wasm-pack` `--target web` bundle. Build it once from
the repository root:

```bash
wasm-pack build bindings/wasm --target web
```

## Serve

ES modules and `fetch()` both need a real HTTP origin, not `file://`. Any static
server from the repository root works:

```bash
# Python:
python -m http.server 8000

# Or Node:
npx http-server -p 8000
```

Then open the demo at `http://localhost:8000/examples/wasm/<file>`. CI cannot open
a browser; it extracts the `<script type="module">` and parses it with
`node --check`, so a broken edit fails there rather than in a reader's tab.

## Demos

| Demo | What it shows |
|------|---------------|

## See also

- [`bindings/wasm/README.md`](../../bindings/wasm/README.md) — install, quick start and the API of the package.
- [`examples/README.md`](../README.md) — the same example in every other language.
