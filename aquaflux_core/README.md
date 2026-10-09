# Aquaflux Core

**More fluid data pipelines** - A high-performance Rust-powered data transformation compiler for Python.

`aquaflux_core` compiles scikit-learn-style pipeline transformers into optimized Rust code, leveraging [Polars](https://pola.rs/) for blazing-fast execution on both Pandas and Polars DataFrames.

## Why Aquaflux?

- **Performance**: Rust-compiled transformations with zero Python overhead
- **Compatibility**: Works seamlessly with both Pandas and Polars DataFrames
- **Declarative**: Define transformations as operations, compile once, execute many times
- **Type-safe**: Rust's type system ensures correctness at compile time
- **Extensible**: Easy to add new operations

## 📦 Installation

```bash
pip install aquaflux-core
```

### Development Installation

```bash
cd aquaflux_core
maturin develop
```

## Quick Start

```python
import aquaflux_core as aquaflux
import pandas as pd

# Create individual operations
select_op = aquaflux.SelectOp(["customer", "order_id", "amount"])
fillna_op = aquaflux.FillNaOp(["customer"], "Unknown")
cast_op = aquaflux.CastOp(["amount"], float)
rename_op = aquaflux.RenameOp(["customer"], ["customer_name"])

# Compile the pipeline
pipeline = aquaflux.compile_pipeline([
    select_op,
    fillna_op,
    cast_op,
    rename_op,
])

# Execute on your data
test_data = pd.DataFrame({
    "customer": ["Alice", None, "Bob"],
    "order_id": [1, 2, 3],
    "amount": ["100.0", "200.0", "300.0"],
})

result = pipeline.execute(test_data)
print(result)
```

## Multi-section graphs

Use named references to connect sections. Declaration order does not determine
execution order: graph construction validates producers and dependencies and
builds topological execution layers.

This self-contained example builds `source -> A -> B -> C` and returns both the
terminal `C` and the intermediate `A`:

```python
import aquaflux_core as aquaflux
import polars as pl

data = pl.DataFrame({
    "customer": ["Alice", "Bob", "Charlie"],
    "amount": ["100.0", "300.0", "200.0"],
})

a = aquaflux.Section([aquaflux.CastOp(["amount"], float)])
a.input_ref = "source"
a.output_ref = "A"

b = aquaflux.Section([
    aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 150.0),
])
b.input_ref = "A"
b.output_ref = "B"

c = aquaflux.Section([aquaflux.SortOp(["amount"], descending=True)])
c.input_ref = "B"
c.output_ref = "C"

graph = aquaflux.ExecutionGraph([c, b, a])
compiled = graph.compile()
assert compiled.input_refs == ["source"]
assert compiled.output_refs == ["C"]

frames = compiled.execute(
    [aquaflux.NamedFrame(data, "source")], optional_extra_outputs=["A"]
)
results = {result.name: result.data for result in frames}
assert [result.name for result in frames] == ["C", "A"]
assert results["C"].rows() == [("Bob", 300.0), ("Charlie", 200.0)]
assert results["A"].rows() == [
    ("Alice", 100.0), ("Bob", 300.0), ("Charlie", 200.0),
]
print(results["C"])
print(results["A"])
```

Every graph section must have an `input_ref` and `output_ref`. Compilation infers
external `input_refs` and terminal `output_refs` (outputs not consumed as primary
or secondary inputs by another section); both properties are read-only.
`output_refs` describes the default terminal outputs, not the full list returned
when extras are requested.

### Graph results and extra outputs

`NamedFrame(data, name)` accepts Pandas or Polars DataFrames. Graph `execute`
always returns a list of `ResultFrame` wrappers, **including when no extra
outputs are requested**. Each wrapper has read-only `name` and `data`
properties: `name` is the actual returned reference name and `data` is a Polars
DataFrame, regardless of the input backend.

- Results follow `compiled.output_refs` order, then distinct
  `optional_extra_outputs` in first-occurrence order.
- Repeated extras and extras that collide with terminal names are deduplicated.
  For the example above, `["A", "A", "C", "B"]` returns names `["C", "A", "B"]`.
- Extras are additive: they do not select a subset of terminals or prune graph
  sections. All inferred external inputs remain required, and all section
  plans are built before output references are resolved.
- An unknown extra reference raises `RuntimeError` during output resolution,
  after section plans have been built (before output DataFrames are collected).
  Earlier input or section-plan errors can therefore occur first.

Migrate graph code from `dict(zip(compiled.output_refs, frames))` to
`results = {result.name: result.data for result in frames}`. Zipping against
`output_refs` would retain wrappers instead of DataFrames and omit extras.
The legacy `aquaflux.compile_pipeline(...).execute(data)` API still returns a
Polars DataFrame directly, not a list of wrappers.

Unproduced primary `input_ref` names and declared auxiliary
`secondary_input_refs` are inferred as external inputs. There is no separate
explicit external-input declaration against which compilation can check typos:
a misspelled dependency that has no producer becomes a required input rather
than a compile-time typo error. Check `compiled.input_refs` and provide every
inferred input, even when requesting only an intermediate as an extra.

Both the graph and compiled object can be reused without consuming their
sections or inputs. See `test_pipeline.py` for branching graphs with shared
intermediates and multiple outputs.

### Reference-based joins

`JoinOp.other` is a string reference name, not an embedded DataFrame. Declare
that reference in the joining section's `secondary_input_refs` so the graph
tracks and schedules the dependency. It can name an external input or an output
produced by another section; internally produced references do not need to be
passed as external `NamedFrame` inputs.

```python
import aquaflux_core as aquaflux
import polars as pl

orders = pl.DataFrame({"customer_id": [1, 2], "amount": [100, 200]})
customers = pl.DataFrame({"customer_id": [1, 2], "customer": ["Alice", "Bob"]})

prepare = aquaflux.Section([aquaflux.SelectOp(["customer_id", "customer"])])
prepare.input_ref = "raw_customers"
prepare.output_ref = "customers"

join = aquaflux.Section([
    aquaflux.JoinOp(
        left_on=["customer_id"],
        right_on=["customer_id"],
        other="customers",
        how="left",
    ),
])
join.input_ref = "orders"
join.secondary_input_refs = ["customers"]
join.output_ref = "enriched_orders"

compiled = aquaflux.ExecutionGraph([join, prepare]).compile()
frames = compiled.execute([
    aquaflux.NamedFrame(orders, "orders"),
    aquaflux.NamedFrame(customers, "raw_customers"),
])
results = {result.name: result.data for result in frames}
assert results["enriched_orders"].columns == ["customer_id", "amount", "customer"]
print(results["enriched_orders"])
```

## Operations Supported

### Currently Implemented

| Operation | Description | Example |
|-----------|-------------|----------|
| `SelectOp` | Select specific columns | `SelectOp(["col1", "col2"])` |
| `FillNaOp` | Fill missing values | `FillNaOp(["col1"], "default")` |
| `CastOp` | Cast column types | `CastOp(["amount"], float)` |
| `RenameOp` | Rename columns | `RenameOp(["old"], ["new"])` |
| `DropOp` | Drop specific columns | `DropOp(["col1", "col2"])` |
| `DropNaOp` | Drop rows with any null values | `DropNaOp()` |
| `SortOp` | Sort by one or more columns | `SortOp(["category", "amount"], descending=False)` |
| `FilterOp` | Filter rows by comparing column to value | `FilterOp("amount", LogicalOp.Gt, 100)` |
| `FilterColOp` | Filter rows by comparing two columns | `FilterColOp("amount", LogicalOp.Gt, "threshold")` |
| `GroupByOp` | Group by columns and aggregate | `GroupByOp(["customer"], [('sale',AggOp.Sum,'sales_sum')])` |
| `WithColumnOp` | Create new columns from expressions | `WithColumns([(Col("a") + Col("b")).alias("sum_ab"),(Col("a") * 2).alias("a_doubled"),])` |

### Sorting

`SortOp(columns, descending)` sorts by the listed columns in priority order,
using later columns to break ties. The `descending` boolean is required:
`False` sorts ascending and `True` sorts descending for **all** listed columns.
Mixed per-column directions are not supported. Both Pandas and Polars inputs
are accepted; `execute` returns a Polars DataFrame in either case.

```python
import aquaflux_core as aquaflux
import polars as pl

data = pl.DataFrame({
    "category": ["B", "A", "A"],
    "amount": [10, 30, 20],
})
for descending in (False, True):
    pipeline = aquaflux.compile_pipeline([
        aquaflux.SortOp(["category", "amount"], descending=descending),
    ])
    result = pipeline.execute(data)
    expected = [("A", 20), ("A", 30), ("B", 10)]
    assert result.rows() == (expected[::-1] if descending else expected)
    print(result)
```

## High Priority Bugs



### 🚧 Planned Operations

**Data Cleaning:**
- (All basic cleaning operations now implemented!)

**Aggregation & Grouping:**
- all done

**Feature Engineering:**
- `WithColumnOp` / `MutateOp` - Create new columns from expressions WIP
  - Example: `total = price * quantity`, `log_amount = log(amount)`
  - THIS IS DONE as a first implementation, support stuff like COL +|*|-|/ COL|INT|FLOAT
  -  but now it can be improved to support scalar string (string right directly detect to columns)

**Scaling & Normalization:**
- `StandardScaleOp` - Standardize features (mean=0, std=1)
- `MinMaxScaleOp` - Scale to range [0, 1]

**Categorical Encoding:**
- `OneHotEncodeOp` - Convert categories to binary columns
- `LabelEncodeOp` - Map categories to integers

**Data Combination:**
- `JoinOp` supports named secondary references, including outputs from other
  graph sections; see [Reference-based joins](#reference-based-joins).


## 🏗️ Architecture

`aquaflux_core` is the core Rust library that provides:

1. **Operation Definitions** (`src/pipeline/`) - Core transformation logic
2. **Python Interface** (`src/interface/`) - PyO3 bindings for Python
3. **Pipeline Compiler** (`src/compiler/`) - Optimizes and compiles operation chains
4. **Execution Engine** - Polars-based execution on DataFrames

### Project Structure

The Aquaflux project consists of two components:

- **`aquaflux-core`** (this package) - Rust-compiled core operations
- **[`aquaflux-fabri`](../aquaflux_fabri/)** - Python helper utilities for pipeline building

`aquaflux-fabri` provides convenience functions and patterns for common pipeline construction tasks, while `aquaflux-core` handles the heavy lifting.

## Development

### Building

```bash
# Install maturin
pip install maturin

# Development build (faster, with debug symbols)
maturin develop

# Release build (optimized)
maturin develop --release
```

### Testing

```bash
# Run Rust tests with all features
cargo test --all-features

# In your existing activated virtual environment, rebuild the Python extension
maturin develop

# Run Python integration tests against that build
python test_pipeline.py
```

### Adding a New Operation

1. Define the operation in `src/pipeline/`
2. Add Python bindings in `src/interface/`
3. Register the operation in `src/lib.rs`
4. Update this README

##  Performance

Aquaflux outperforms both Pandas and Polars (called from Python) by compiling pipelines into optimized Rust code with lazy evaluation.

### Benchmark Results (1M rows)

| Operation | Pandas | Polars | Aquaflux | Winner |
|-----------|--------|--------|----------|--------|
| **Basic Pipeline** | 156.0ms | 25.1ms | **22.5ms** | ✅ Aquaflux |
| **Complex Pipeline** | 200.2ms | 35.2ms | **26.0ms** | ✅ Aquaflux |
| **GroupBy** | 20.3ms | **7.9ms** | 8.7ms | Polars |
| **WithColumns** | 3.3ms | 0.73ms | **0.64ms** | ✅ Aquaflux |

Aquaflux beats Polars-from-Python by:
- **11%** on Basic Pipeline
- **26%** on Complex Pipeline  
- **12%** on WithColumns

### Why faster than Polars from Python?

1. **Single lazy plan**: Entire pipeline compiled into one optimized Rust query
2. **Reduced PyO3 overhead**: One Python↔Rust boundary crossing per execution
3. **Full optimization**: Polars sees the complete pipeline for predicate/projection pushdown

- **Polars** - The underlying DataFrame library
- **PyO3** - Rust-Python bindings
- **scikit-learn** - Inspiration for the pipeline API

## Idea of flow

For each transformer, AquaFlux attempts to automatically translate it into a native AquaFlux instruction. If no translation is available, a user-defined translation can be provided. As a final fallback, the transformer is ahead-of-time compiled into native machine code and embedded into the execution pipeline, avoiding runtime interpretation overhead.
