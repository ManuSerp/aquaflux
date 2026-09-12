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

# Test JoinOp
print("\n--- JoinOp Test ---")

# Create left DataFrame (orders)
orders_df = pandas.DataFrame({
    "order_id": [1, 2, 3, 4],
    "customer_id": [101, 102, 101, 103],
    "amount": [250.0, 150.0, 300.0, 450.0],
})

# Create right DataFrame (customers)
customers_df = pandas.DataFrame({
    "id": [101, 102, 104],
    "name": ["Alice", "Bob", "Diana"],
    "region": ["North", "South", "East"],
})

print("Orders DataFrame (left):")
print(orders_df)
print("\nCustomers DataFrame (right):")
print(customers_df)

# Inner join: only matching rows
join_inner_op = aquaflux.JoinOp(
    left_on=["customer_id"],
    right_on=["id"],
    other=customers_df,
    how="inner"
)

pipeline_join_inner = aquaflux.compile_pipeline([join_inner_op])
result_join_inner = pipeline_join_inner.execute(orders_df)

print("\nInner Join Result (orders with matching customers):")
print(result_join_inner)

# Left join: all orders, with customer info where available
join_left_op = aquaflux.JoinOp(
    left_on=["customer_id"],
    right_on=["id"],
    other=customers_df,
    how="left"
)

pipeline_join_left = aquaflux.compile_pipeline([join_left_op])
result_join_left = pipeline_join_left.execute(orders_df)

print("\nLeft Join Result (all orders, customers where available):")
print(result_join_left)

# Test SortOp with both pandas and polars inputs (output is always polars).
print("\n--- SortOp Test ---")

sort_data = {
    "category": ["B", "A", "B", "A"],
    "amount": [20, 30, 10, 20],
    "order_id": [1, 2, 3, 4],
}

for dataframe_type in (pandas.DataFrame, polars.DataFrame):
    test_data_sort = dataframe_type(sort_data)
    for descending in (False, True):
        pipeline_sort = aquaflux.compile_pipeline([
            aquaflux.SortOp(["order_id"], descending=descending),
        ])
        result_sort = pipeline_sort.execute(test_data_sort)
        assert isinstance(result_sort, polars.DataFrame)
        expected_ids = [4, 3, 2, 1] if descending else [1, 2, 3, 4]
        assert result_sort["order_id"].to_list() == expected_ids

        # Repeated categories exercise the secondary key in both directions.
        pipeline_sort_multi = aquaflux.compile_pipeline([
            aquaflux.SortOp(["category", "amount"], descending=descending),
        ])
        result_sort_multi = pipeline_sort_multi.execute(test_data_sort)
        assert isinstance(result_sort_multi, polars.DataFrame)
        expected_rows = [("A", 20, 4), ("A", 30, 2), ("B", 10, 3), ("B", 20, 1)]
        if descending:
            expected_rows.reverse()
        assert result_sort_multi.rows() == expected_rows

        print(f"\nSort Result ({dataframe_type.__module__}, descending={descending}):")
        print(result_sort_multi)
