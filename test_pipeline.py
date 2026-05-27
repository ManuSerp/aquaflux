#!/usr/bin/env python3
"""Test script demonstrating the aquaflux pipeline compilation."""

import aquaflux
import pandas
import polars

# Create individual operations
select_op = aquaflux.SelectOp(["customer", "order_id", "amount"])
# fillna_op = aquaflux.FillNaOp(["customer"], aquaflux.ScalarValue.String("Unknown"))
fillna_op = aquaflux.FillNaOp(["customer"], "Unknown")
# Using Python type directly instead of aquaflux.DataType.Float64
cast_op = aquaflux.CastOp(["amount"], float)
rename_op = aquaflux.RenameOp(["customer"], ["customer_name"])

# Compile the pipeline
pipeline = aquaflux.compile_pipeline(
    [
        select_op,
        fillna_op,
        cast_op,
        rename_op,
    ]
)

print(f" Successfully compiled pipeline: {pipeline}")

test_data = pandas.DataFrame(
    {
        "customer": ["Alice", None, "Bob"],
        "order_id": [1, 2, 3],
        "amount": ["100.0", "200.0", "300.0"],
    }
)

result = pipeline.execute(test_data)

print("\n Pipeline Execution Result:")
print(result)
