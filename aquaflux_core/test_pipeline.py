#!/usr/bin/env python3
"""Demonstrate reusable Sections and execution graphs alongside legacy examples."""

import aquaflux_core as aquaflux
import pandas
import polars

# Legacy API: kept as a compatibility example; all examples below use Section.
print("\n--- Legacy Pipeline Test ---")
legacy_data = pandas.DataFrame({"amount": [100, 200, 300]})
legacy_ops = [aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 150)]
legacy_pipeline = aquaflux.compile_pipeline(legacy_ops)
legacy_result = legacy_pipeline.execute(legacy_data)
assert isinstance(legacy_result, polars.DataFrame)
assert legacy_result["amount"].to_list() == [200, 300]
assert aquaflux.Section(legacy_ops).compile().execute(legacy_data).equals(legacy_result)
print(legacy_result)

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

# Declare the operation chain as a Section, then compile it.
section_transform = aquaflux.Section(
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

section_transform.name = "clean_orders"
pipeline = section_transform.compile()

print(f" Successfully compiled section: {pipeline}")

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

section_groupby = aquaflux.Section([groupby_op])
pipeline_groupby = section_groupby.compile()
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

# Test WithColumns in a Section
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

section_mut = aquaflux.Section([with_cols_op])
pipeline_mut = section_mut.compile()
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

section_join_inner = aquaflux.Section([join_inner_op])
pipeline_join_inner = section_join_inner.compile()
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

section_join_left = aquaflux.Section([join_left_op])
pipeline_join_left = section_join_left.compile()
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
        section_sort = aquaflux.Section([
            aquaflux.SortOp(["order_id"], descending=descending),
        ])
        pipeline_sort = section_sort.compile()
        result_sort = pipeline_sort.execute(test_data_sort)
        assert isinstance(result_sort, polars.DataFrame)
        expected_ids = [4, 3, 2, 1] if descending else [1, 2, 3, 4]
        assert result_sort["order_id"].to_list() == expected_ids

        # Repeated categories exercise the secondary key in both directions.
        section_sort_multi = aquaflux.Section([
            aquaflux.SortOp(["category", "amount"], descending=descending),
        ])
        pipeline_sort_multi = section_sort_multi.compile()
        result_sort_multi = pipeline_sort_multi.execute(test_data_sort)
        assert isinstance(result_sort_multi, polars.DataFrame)
        expected_rows = [("A", 20, 4), ("A", 30, 2), ("B", 10, 3), ("B", 20, 1)]
        if descending:
            expected_rows.reverse()
        assert result_sort_multi.rows() == expected_rows

        print(f"\nSort Result ({dataframe_type.__module__}, descending={descending}):")
        print(result_sort_multi)

# Declare a Section, then compile and execute it separately.
print("\n--- Section Test ---")

section_data = pandas.DataFrame({
    "customer": ["Alice", None, "Bob"],
    "amount": ["100.0", "200.0", "300.0"],
    "status": ["active", "pending", "active"],
})
section_ops = [
    aquaflux.SelectOp(["customer", "amount"]),
    aquaflux.FillNaOp(["customer"], "Unknown"),
    aquaflux.CastOp(["amount"], float),
    aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 150.0),
    aquaflux.RenameOp(["customer"], ["customer_name"]),
    aquaflux.SortOp(["amount"], descending=True),
]

section = aquaflux.Section(section_ops)
section.name = "high_value_customers"
compiled_section = section.compile()
result_section = compiled_section.execute(section_data)

assert isinstance(compiled_section, aquaflux.CompiledSection)
assert isinstance(result_section, polars.DataFrame)
assert result_section.columns == ["customer_name", "amount"]
assert result_section.rows() == [("Bob", 300.0), ("Unknown", 200.0)]


# Compilation does not consume the section's stored operations.
assert section.compile().execute(section_data).equals(result_section)

print(f"\nCompiled Section ({section.name}): {compiled_section}")
print(result_section)


# Shared intermediate: orders -> cleaned -> {high_value, sorted_orders}.
print("\n--- ExecutionGraph Test ---")

high_value = aquaflux.Section([
    aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 150.0),
])
high_value.name = "filter_high_value"
high_value.input_ref = "cleaned"
high_value.output_ref = "high_value"

sorted_orders = aquaflux.Section([
    aquaflux.SortOp(["amount"], descending=True),
])
sorted_orders.name = "sort_orders"
sorted_orders.input_ref = "cleaned"
sorted_orders.output_ref = "sorted_orders"

clean_orders = aquaflux.Section([
    aquaflux.SelectOp(["customer", "amount"]),
    aquaflux.CastOp(["amount"], float),
])
clean_orders.name = "clean_orders"
clean_orders.input_ref = "orders"
clean_orders.output_ref = "cleaned"

# Consumers deliberately precede their producer; graph layers determine execution.
graph = aquaflux.ExecutionGraph([high_value, sorted_orders, clean_orders])
compiled_graph = graph.compile()
graph_data = {
    "customer": ["Alice", "Bob", "Charlie"],
    "amount": ["100.0", "300.0", "200.0"],
}
expected_graph_rows = {
    "high_value": [("Bob", 300.0), ("Charlie", 200.0)],
    "sorted_orders": [("Bob", 300.0), ("Charlie", 200.0), ("Alice", 100.0)],
}

# Both the graph and its compiled form are reusable, across dataframe backends.
for executable_graph in (compiled_graph, graph.compile()):
    assert isinstance(executable_graph, aquaflux.CompiledExecutionGraph)
    assert isinstance(executable_graph.input_refs, list)
    assert executable_graph.input_refs == ["orders"]
    assert isinstance(executable_graph.output_refs, list)
    assert all(isinstance(ref, str) for ref in executable_graph.output_refs)
    assert len(executable_graph.output_refs) == 2
    assert set(executable_graph.output_refs) == set(expected_graph_rows)
    for dataframe_type in (pandas.DataFrame, polars.DataFrame):
        named_orders = aquaflux.NamedFrame(dataframe_type(graph_data), "orders")
        for _ in range(2):
            graph_results = executable_graph.execute([named_orders])
            assert isinstance(graph_results, list)
            assert len(graph_results) == len(executable_graph.output_refs)
            for ref, frame in zip(executable_graph.output_refs, graph_results):
                assert isinstance(frame, polars.DataFrame)
                assert frame.columns == ["customer", "amount"]
                assert frame.rows() == expected_graph_rows[ref]


def assert_graph_error(action, error_type=Exception):
    # Catch Python errors only: a native panic must not pass a negative check.
    try:
        action()
    except error_type:
        pass
    else:
        raise AssertionError(f"Expected {error_type.__name__}")


for attribute in ("input_refs", "output_refs"):
    assert_graph_error(lambda: setattr(compiled_graph, attribute, []), AttributeError)

named_orders = aquaflux.NamedFrame(polars.DataFrame(graph_data), "orders")
unexpected_orders = aquaflux.NamedFrame(polars.DataFrame(graph_data), "unexpected")
assert_graph_error(lambda: compiled_graph.execute([]))
assert_graph_error(lambda: compiled_graph.execute([named_orders, named_orders]))
assert_graph_error(lambda: compiled_graph.execute([named_orders, unexpected_orders]))
assert_graph_error(lambda: aquaflux.NamedFrame(object(), "orders"))
assert_graph_error(lambda: compiled_graph.execute([object()]))

# Missing references are rejected by compile(), not by graph construction.
for missing_ref in ("input_ref", "output_ref"):
    incomplete = aquaflux.Section([aquaflux.SelectOp(["amount"])])
    incomplete.name = "incomplete"
    if missing_ref != "input_ref":
        incomplete.input_ref = "orders"
    if missing_ref != "output_ref":
        incomplete.output_ref = "result"
    incomplete_graph = aquaflux.ExecutionGraph([incomplete])
    assert_graph_error(incomplete_graph.compile, ValueError)

# Duplicate producers and cycles may be rejected during graph construction.
duplicate = aquaflux.Section([aquaflux.SelectOp(["amount"])])
duplicate.name = "duplicate_producer"
duplicate.input_ref = "orders"
duplicate.output_ref = "cleaned"
assert_graph_error(
    lambda: aquaflux.ExecutionGraph([clean_orders, duplicate]).compile(), ValueError
)

cycle = aquaflux.Section([aquaflux.SelectOp(["amount"])])
cycle.name = "cycle"
cycle.input_ref = "cleaned"
cycle.output_ref = "orders"
assert_graph_error(
    lambda: aquaflux.ExecutionGraph([clean_orders, cycle]).compile(), ValueError
)

print("ExecutionGraph assertions passed")


# A complete example: clean and enrich once, then build two different reports.
print("\n--- Example: Two Reports from the Same Orders ---")
print(
    "orders -> cleaned_orders -> priced_orders\n"
    "                              |\n"
    "                              +-> customer_sales\n"
    "                              +-> high_value_orders"
)

example_orders = polars.DataFrame({
    "order_id": [1, 2, 3, 4, 5, 6],
    "customer": ["Alice", "Bob", "Alice", "Charlie", "Bob", None],
    "unit_price": ["20.0", "80.0", "15.0", "120.0", "50.0", "30.0"],
    "quantity": [3, 2, 4, 1, 5, 2],
})
print("\nInput orders:")
print(example_orders)

example_clean = aquaflux.Section([
    aquaflux.FillNaOp(["customer"], "Unknown"),
    aquaflux.CastOp(["unit_price"], float),
])
example_clean.name = "clean_order_data"
example_clean.input_ref = "orders"
example_clean.output_ref = "cleaned_orders"

example_price = aquaflux.Section([
    aquaflux.WithColumns([
        (aquaflux.Col("unit_price") * aquaflux.Col("quantity")).alias("order_total"),
    ]),
])
example_price.name = "calculate_order_totals"
example_price.input_ref = "cleaned_orders"
example_price.output_ref = "priced_orders"

# Branch 1: aggregate all orders into a customer-level sales leaderboard.
example_summary = aquaflux.Section([
    aquaflux.GroupByOp(
        group_columns=["customer"],
        aggregations=[
            ("order_total", aquaflux.AggOp.Sum, "total_sales"),
            ("order_total", aquaflux.AggOp.Mean, "average_order"),
            ("quantity", aquaflux.AggOp.Sum, "items_sold"),
        ],
    ),
    aquaflux.SortOp(["total_sales", "customer"], descending=True),
])
example_summary.name = "summarize_customer_sales"
example_summary.input_ref = "priced_orders"
example_summary.output_ref = "customer_sales"

# Branch 2: keep individual high-value orders instead of aggregating them.
example_high_value = aquaflux.Section([
    aquaflux.FilterOp("order_total", aquaflux.LogicalOp.Gt, 150.0),
    aquaflux.SelectOp(["order_id", "customer", "quantity", "order_total"]),
    aquaflux.SortOp(["order_total"], descending=True),
])
example_high_value.name = "report_high_value_orders"
example_high_value.input_ref = "priced_orders"
example_high_value.output_ref = "high_value_orders"

# Deliberately declare the reports first: references determine execution order.
example_graph = aquaflux.ExecutionGraph([
    example_summary,
    example_high_value,
    example_price,
    example_clean,
]).compile()

# Intermediate references stay inside the lazy plans; only the reports are returned.
example_frames = example_graph.execute([
    aquaflux.NamedFrame(example_orders, "orders"),
])
example_results = dict(zip(example_graph.output_refs, example_frames))

print("\nCustomer sales leaderboard (all orders, grouped by customer):")
print(example_results["customer_sales"])
print("\nHigh-value orders (individual orders with a total above 150):")
print(example_results["high_value_orders"])
