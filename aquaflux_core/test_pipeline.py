#!/usr/bin/env python3
"""Test script demonstrating the aquaflux pipeline compilation."""

import aquaflux_core as aquaflux
import pandas
import polars

# Create individual operations
select_op = aquaflux.SelectOp(["customer", "order_id", "amount", "status"])
# fillna_op = aquaflux.FillNaOp(["customer"], aquaflux.ScalarValue.String("Unknown"))
fillna_op = aquaflux.FillNaOp(["customer"], "Unknown")
# Using Python type directly instead of aquaflux.DataType.Float64
cast_op = aquaflux.CastOp(["amount"], float)
rename_op = aquaflux.RenameOp(["customer"], ["customer_name"])

# New operations:
# Drop specific columns
drop_op = aquaflux.DropOp(["status"])
# Drop rows with any null values
drop_na_op = aquaflux.DropNaOp()

# Compile the pipeline
pipeline = aquaflux.compile_pipeline(
    [
        select_op,
        fillna_op,      # Fills nulls in 'customer' column (row 2)
        cast_op,
        rename_op,
        drop_op,        # Remove the 'status' column
        drop_na_op,     # Drop rows with remaining nulls (row 4 with null order_id)
    ]
)

print(f" Successfully compiled pipeline: {pipeline}")

test_data = pandas.DataFrame(
    {
        "customer": ["Alice", None, "Bob", "Charlie"],
        "order_id": [1, 2, 3, None],  # Row 4 has a null here
        "amount": ["100.0", "200.0", "300.0", "400.0"],
        "status": ["active", "pending", "active", "completed"],
    }
)

print("\nInput Data:")
print(test_data)

result = pipeline.execute(test_data)

print("\n Pipeline Execution Result:")
print(result)
