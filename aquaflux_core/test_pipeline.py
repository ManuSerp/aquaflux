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

# Filter operations:
# Filter rows where amount > 150 (keeps only rows with amount > 150)
filter_op = aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 150.0)

# FilterColOp example: compare two columns
# Add a minimum threshold column to test data first
# Filter rows where amount > min_threshold
filter_col_op = aquaflux.FilterColOp("amount", aquaflux.LogicalOp.Gt, "min_threshold")

# Compile the pipeline
pipeline = aquaflux.compile_pipeline(
    [
        select_op,
        fillna_op,      # Fills nulls in 'customer' column (row 2)
        cast_op,
        filter_op,      # Filter amount > 150 (keeps rows 2, 3, 4)
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
        "min_threshold": [50.0, 150.0, 250.0, 350.0],
    }
)

print("\nInput Data:")
print(test_data)

result = pipeline.execute(test_data)

print("\n Pipeline Execution Result:")
print(result)

# GroupBy operation
groupby_op = aquaflux.GroupByOp(
    group_columns=["category"],
    aggregations=[
        ("sales", aquaflux.AggOp.Sum, "total_sales"),
        ("sales", aquaflux.AggOp.Mean, "avg_sales"),
    ],
)

test_data_groupby = pandas.DataFrame(
    {
        "category": ["A", "B", "A", "B", "A"],
        "sales": [100.0, 200.0, 150.0, 300.0, 120.0],
    }
)

print("\nGroupBy Test Data:")
print(test_data_groupby)

pipeline_groupby = aquaflux.compile_pipeline([groupby_op])
result_groupby = pipeline_groupby.execute(test_data_groupby)

print("\nGroupBy Result:")
print(result_groupby)


# Test Col expressions and WithColumns
print("\n--- Col Expression Test ---")

from aquaflux_core import Col, WithColumns

# Test Col("col1") + Col("col2")
mut1 = Col("a") + Col("b")
print(f"Col('a') + Col('b') -> string_expr: '{mut1.string_expr}'")

# Test Col("col1") - 2 (column minus literal)
mut2 = Col("a") - 2
print(f"Col('a') - 2 -> string_expr: '{mut2.string_expr}'")

# Test with alias
mut3 = (Col("price") * Col("quantity")).alias("total")
print(f"(Col('price') * Col('quantity')).alias('total') -> string_expr: '{mut3.string_expr}', alias: '{mut3.alias}'")

# Test WithColumns in a pipeline
test_data_mut = pandas.DataFrame({
    "a": [1, 2, 3],
    "b": [10, 20, 30],
})

print("\nWithColumns Test Data:")
print(test_data_mut)

with_cols_op = WithColumns([
    (Col("a") + Col("b")).alias("sum_ab"),
    (Col("a") * 2).alias("a_doubled"),
    (1 - Col("a")).alias("testlefneg"),
    (Col("a") * 1.5).alias("testfloat"),
])

pipeline_mut = aquaflux.compile_pipeline([with_cols_op])
result_mut = pipeline_mut.execute(test_data_mut)

print("\nWithColumns Result:")
print(result_mut)
