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

### Pipelines as transparent, inspectable objects

Aquaflux is designed to do more than run a flat sequence of low-level transformations. A pipeline should provide a concrete, hierarchical representation of the complete data-processing flow: named transformers composed of operations or nested transformers, their configuration and metadata, and how data moves from input to output. This raises the representation from engine instructions such as select, with-columns, and drop to the domain-level transformers users actually reason about.

The compiler will be able to lower this hierarchy into an optimized executable plan while preserving the logical transformer boundaries for tooling. Planned metadata will let users name pipelines and transformers, attach descriptions or tags, retrieve their steps, and inspect useful information such as affected columns, schemas, stable paths, and execution details.

Aquaflux also plans to provide opt-in execution tools for validation and troubleshooting:

- **Result comparison**: compare outputs across pipeline implementations, configurations, or runs.
- **Hierarchical debug mode**: inspect results first at named pipeline or transformer boundaries, then drill down into a selected transformer instruction by instruction.
- **Rich metadata**: use pipeline and transformer identity, structure, and step information to power diagnostics, profiling, reporting, visualization, and future tooling.

These capabilities are part of the project direction and are not all available yet. See [MAN-18](https://linear.app/manuserp/issue/MAN-18/add-pipeline-introspection-result-comparison-and-step-by-step-debug) for observability and [MAN-19](https://linear.app/manuserp/issue/MAN-19/support-hierarchical-pipelines-of-named-composite-transformers) for hierarchical transformer pipelines.

## ⚡ Performance

Aquaflux outperforms both Pandas and Polars (called from Python) by compiling pipelines into optimized Rust code with lazy evaluation.

### Benchmark Results (1M rows)

| Operation | Pandas | Polars | Aquaflux | Winner |
|-----------|--------|--------|----------|--------|
| **Basic Pipeline** | 156.0ms | 25.1ms | **22.5ms** | ✅ Aquaflux (11% faster than Polars) |
| **Complex Pipeline** | 200.2ms | 35.2ms | **26.0ms** | ✅ Aquaflux (26% faster than Polars) |
| **GroupBy** | 20.3ms | **7.9ms** | 8.7ms | Polars |
| **WithColumns** | 3.3ms | 0.73ms | **0.64ms** | ✅ Aquaflux (12% faster than Polars) |

<details>
<summary>Full benchmark results (all data sizes)</summary>

#### Basic Pipeline (Select, FillNa, Cast, Filter, Rename, DropNa)
| Data Size | Pandas | Polars | Aquaflux | Fastest |
|-----------|--------|--------|----------|--------|
| 1,000 | 0.82ms | 0.35ms | **0.32ms** | Aquaflux |
| 10,000 | 2.07ms | 0.58ms | **0.55ms** | Aquaflux |
| 100,000 | 15.96ms | **2.48ms** | 2.50ms | Polars |
| 1,000,000 | 155.99ms | 25.10ms | **22.45ms** | Aquaflux |

#### Complex Pipeline (FillNa, Cast, Filter, WithColumns, Select, GroupBy)
| Data Size | Pandas | Polars | Aquaflux | Fastest |
|-----------|--------|--------|----------|--------|
| 1,000 | 2.39ms | 0.57ms | **0.42ms** | Aquaflux |
| 10,000 | 4.09ms | 0.83ms | **0.72ms** | Aquaflux |
| 100,000 | 21.58ms | 3.65ms | **2.91ms** | Aquaflux |
| 1,000,000 | 200.19ms | 35.23ms | **25.96ms** | Aquaflux |

#### GroupBy Aggregation
| Data Size | Pandas | Polars | Aquaflux | Fastest |
|-----------|--------|--------|----------|--------|
| 1,000 | 1.37ms | 0.25ms | **0.23ms** | Aquaflux |
| 10,000 | 1.47ms | **0.25ms** | 0.31ms | Polars |
| 100,000 | 3.22ms | 1.09ms | **1.01ms** | Aquaflux |
| 1,000,000 | 20.25ms | **7.92ms** | 8.66ms | Polars |

#### WithColumns (Computed Columns)
| Data Size | Pandas | Polars | Aquaflux | Fastest |
|-----------|--------|--------|----------|--------|
| 1,000 | 0.36ms | **0.11ms** | 0.15ms | Polars |
| 10,000 | 0.38ms | **0.10ms** | 0.13ms | Polars |
| 100,000 | 0.60ms | 0.17ms | **0.16ms** | Aquaflux |
| 1,000,000 | 3.29ms | 0.73ms | **0.64ms** | Aquaflux |

</details>

### Why is Aquaflux faster than Polars from Python?

1. **Single lazy plan**: The entire pipeline is compiled into one optimized query plan in Rust
2. **Reduced PyO3 overhead**: Only one Python↔Rust boundary crossing per execution
3. **Better optimization**: Polars' query optimizer sees the full pipeline and applies predicate/projection pushdown

##  Documentation

See individual package READMEs:
- [aquaflux-core](./aquaflux_core/README.md) - Core operations and architecture
- [aquaflux-fabri](./aquaflux_fabri/README.md) - Helper utilities
