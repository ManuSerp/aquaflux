# Aquaflux

**More fluid data pipelines** - A high-performance Rust-powered data transformation compiler for Python.

Aquaflux compiles scikit-learn-style pipeline transformers into optimized Rust code for blazing-fast data transformations.

##  Project Structure

This monorepo contains two complementary packages:

### [`aquaflux-core`](./aquaflux_core/)

The core Rust library providing high-performance data transformation operations:
- Rust-compiled operations (Select, FillNa, Cast, Rename, etc.)
- Polars-based execution engine
- PyO3 Python bindings
- Zero-copy DataFrame interop with Pandas and Polars

**Installation:**
```bash
pip install aquaflux-core
```

### [`aquaflux-fabri`](./aquaflux_fabri/)

Python helper utilities for pipeline building:
- Convenience functions for common pipeline patterns
- Scikit-learn pipeline converters
- Higher-level abstractions over `aquaflux-core`

**Installation:**
```bash
pip install aquaflux-fabri
```

## Quick Start

```python
import aquaflux_core as aquaflux
import pandas as pd

# Define and compile a pipeline
pipeline = aquaflux.compile_pipeline([
    aquaflux.SelectOp(["customer", "amount"]),
    aquaflux.FillNaOp(["customer"], "Unknown"),
    aquaflux.CastOp(["amount"], float),
])

# Execute on your data
data = pd.DataFrame({"customer": ["Alice", None], "amount": ["100", "200"]})
result = pipeline.execute(data)
```

##  Why Aquaflux?

- ** Fast**: Rust-compiled transformations with zero Python overhead
- ** Compatible**: Works with both Pandas and Polars DataFrames
- ** Declarative**: Define transformations as composable operations
- ** Type-safe**: Rust's type system catches errors at compile time
- ** Extensible**: Easy to add custom operations

##  Documentation

See individual package READMEs:
- [aquaflux-core](./aquaflux_core/README.md) - Core operations and architecture
- [aquaflux-fabri](./aquaflux_fabri/README.md) - Helper utilities
