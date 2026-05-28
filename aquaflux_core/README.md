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

### 🚧 Planned Operations

**Data Cleaning:**
- (All basic cleaning operations now implemented!)

**Aggregation & Grouping:**
- `GroupByOp` - Group by columns with aggregations (sum, mean, count, etc.)

**Feature Engineering:**
- `WithColumnOp` / `MutateOp` - Create new columns from expressions
  - Example: `total = price * quantity`, `log_amount = log(amount)`

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

By compiling transformations to Rust and using Polars under the hood, `aquaflux-core` can be significantly faster than pure Python implementations, especially for:

- Large datasets (> 1M rows)
- Complex pipeline chains
- Repeated transformations on similar data

- **Polars** - The underlying DataFrame library
- **PyO3** - Rust-Python bindings
- **scikit-learn** - Inspiration for the pipeline API
