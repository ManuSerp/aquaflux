# Aquaflux Fabri

**Python helpers for fluid pipeline construction**

`aquaflux-fabri` provides convenience functions and high-level utilities to simplify building data transformation pipelines with [`aquaflux-core`](../aquaflux_core/).

## Installation

```bash
pip install aquaflux-fabri aquaflux-core
```

## What is Fabri?

"Fabri" provides the tools and patterns to craft data pipelines more ergonomically. While `aquaflux-core` gives you raw building blocks (operations), `aquaflux-fabri` gives you:

- **Pre-built pipeline patterns** for common use cases
- **Scikit-learn converters** to transform existing sklearn pipelines
- **Helper functions** to reduce boilerplate
- **Validation utilities** for pipeline configuration

## Quick Start

```python
import aquaflux_core as aquaflux
from aquaflux_fabri import create_preprocessing_pipeline

# Instead of manually building operations...
pipeline = create_preprocessing_pipeline(
    numeric_cols=["amount", "price"],
    categorical_cols=["customer", "category"]
)

# Execute
result = pipeline.execute(data)
```

## Available Helpers

### Pipeline Builders

```python
from aquaflux_fabri.helpers import (
    create_preprocessing_pipeline,
    # More to come...
)
```

**`create_preprocessing_pipeline(numeric_cols, categorical_cols)`**
- Fills missing values (0.0 for numeric, "Unknown" for categorical)
- Casts numeric columns to float
- Returns compiled pipeline

### Scikit-learn Converters (Planned)

```python
from aquaflux_fabri import sklearn_to_aquaflux
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler

sklearn_pipeline = Pipeline([
    ('scaler', StandardScaler()),
    # ...
])

# Convert to aquaflux
aquaflux_pipeline = sklearn_to_aquaflux(sklearn_pipeline)
```

## Examples

See [`exemple.py`](./exemple.py) for example scikit-learn pipeline conversions.

## Architecture

`aquaflux-fabri` is pure Python code that:
1. Wraps `aquaflux-core` operations
2. Provides domain-specific abstractions
3. Handles common patterns and edge cases
4. No performance overhead - just convenience

### Relationship to aquaflux_core

```
┌─────────────────────────────────────┐
│      Your Application Code          │
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      aquaflux-fabri (Python)        │  ← Convenience & Patterns
│  • Helper functions                 │
│  • Pipeline builders                │
│  • sklearn converters               │
└─────────────┬───────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│      aquaflux-core (Rust)           │  ← Performance & Execution
│  • Core operations                  │
│  • Pipeline compiler                │
│  • Polars execution engine          │
└─────────────────────────────────────┘
```

## Development

This is a pure Python package - no build step required:

```bash
# Install in editable mode
pip install -e aquaflux-fabri/

# Or just use directly
export PYTHONPATH="$PYTHONPATH:$(pwd)/aquaflux_fabri"
```
