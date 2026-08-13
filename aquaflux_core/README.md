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
| `FilterOp` | Filter rows by comparing column to value | `FilterOp("amount", LogicalOp.Gt, 100)` |
| `FilterColOp` | Filter rows by comparing two columns | `FilterColOp("amount", LogicalOp.Gt, "threshold")` |
| `GroupByOp` | Group by columns and aggregate | `GroupByOp(["customer"], [('sale',AggOp.Sum,'sales_sum')])` |
| `WithColumnOp` | Create new columns from expressions | `WithColumns([(Col("a") + Col("b")).alias("sum_ab"),(Col("a") * 2).alias("a_doubled"),])` |

## High Priority Bugs

1. **Literal on left operand not supported**
   - `(Col("amount") * (1 - Col("discount"))).alias("discounted_value")` doesn't work
   - We need to accept literal in left operand, not expect only columns


### 🚧 Planned Operations

**Data Cleaning:**
- (All basic cleaning operations now implemented!)

**Aggregation & Grouping:**
- all done

**Feature Engineering:**
- `WithColumnOp` / `MutateOp` - Create new columns from expressions WIP
  - Example: `total = price * quantity`, `log_amount = log(amount)`
  - THIS IS DONE as a first implementation, support stuff like COL +|*|-|/ COL|INT
  -  but now it can be improved to support scalar float and scalar string (string right directly detect to columns)

**Scaling & Normalization:**
- `StandardScaleOp` - Standardize features (mean=0, std=1)
- `MinMaxScaleOp` - Scale to range [0, 1]

**Categorical Encoding:**
- `OneHotEncodeOp` - Convert categories to binary columns
- `LabelEncodeOp` - Map categories to integers

**Data Combination:**
- `JoinOp` - Merge datasets (left, inner, outer joins)
- `SortOp` - Sort by columns

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
# Run Rust tests
cargo test

# Run Python integration tests
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
