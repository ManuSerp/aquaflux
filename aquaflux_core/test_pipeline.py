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

# Secondary frames are resolved by the reference name used in JoinOp.other.
secondary_customers = [aquaflux.NamedFrame(customers_df, "customers")]

# Inner join: only matching rows
join_inner_op = aquaflux.JoinOp(
    left_on=["customer_id"],
    right_on=["id"],
    other="customers",
    how="inner"
)

section_join_inner = aquaflux.Section([join_inner_op])
section_join_inner.secondary_input_refs = ["customers"]
pipeline_join_inner = section_join_inner.compile()
result_join_inner = pipeline_join_inner.execute(orders_df, secondary_customers)

print("\nInner Join Result (orders with matching customers):")
print(result_join_inner)

# Left join: all orders, with customer info where available
join_left_op = aquaflux.JoinOp(
    left_on=["customer_id"],
    right_on=["id"],
    other="customers",
    how="left"
)

section_join_left = aquaflux.Section([join_left_op])
section_join_left.secondary_input_refs = ["customers"]
pipeline_join_left = section_join_left.compile()
result_join_left = pipeline_join_left.execute(orders_df, secondary_customers)

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
            for ref, result in zip(executable_graph.output_refs, graph_results):
                assert isinstance(result, aquaflux.ResultFrame)
                assert result.name == ref
                assert isinstance(result.data, polars.DataFrame)
                assert result.data.columns == ["customer", "amount"]
                assert result.data.rows() == expected_graph_rows[result.name]

extra_results = compiled_graph.execute(
    [aquaflux.NamedFrame(polars.DataFrame(graph_data), "orders")],
    optional_extra_outputs=["cleaned", "high_value"],
)
assert [result.name for result in extra_results] == compiled_graph.output_refs + ["cleaned"]
assert all(isinstance(result, aquaflux.ResultFrame) for result in extra_results)
assert extra_results[-1].data.rows() == [
    ("Alice", 100.0), ("Bob", 300.0), ("Charlie", 200.0),
]


# MAN-38: source -> A -> B -> C, with a distinct schema at every stage.
print("\n--- Extra Output Contract Test ---")
extra_a = aquaflux.Section([
    aquaflux.SelectOp(["id", "value"]),
    aquaflux.RenameOp(["value"], ["a"]),
])
extra_a.input_ref = "source"
extra_a.output_ref = "A"
extra_b = aquaflux.Section([
    aquaflux.SelectOp(["a"]),
    aquaflux.RenameOp(["a"], ["b"]),
])
extra_b.input_ref = "A"
extra_b.output_ref = "B"
extra_c = aquaflux.Section([aquaflux.RenameOp(["b"], ["c"])])
extra_c.input_ref = "B"
extra_c.output_ref = "C"
extra_compiled = aquaflux.ExecutionGraph([extra_a, extra_b, extra_c]).compile()
extra_source = aquaflux.NamedFrame(polars.DataFrame({
    "id": [2, 1], "value": [20, 10], "unused": [0, 0],
}), "source")
extra_expected = {
    "A": polars.DataFrame({"id": [1, 2], "a": [10, 20]}),
    "B": polars.DataFrame({"b": [10, 20]}),
    "C": polars.DataFrame({"c": [10, 20]}),
}
extra_default_refs = extra_compiled.output_refs.copy()
assert extra_default_refs == ["C"]
for extras, expected_names in (
    (None, ["C"]),
    ([], ["C"]),
    (["A"], ["C", "A"]),
    (["B", "A"], ["C", "B", "A"]),
    (["A", "A"], ["C", "A"]),
    (["C", "A", "C", "A"], ["C", "A"]),
    (None, ["C"]),
):
    frames = extra_compiled.execute([extra_source], optional_extra_outputs=extras)
    assert [result.name for result in frames] == expected_names
    assert len(frames) == len(set(expected_names))
    for result in frames:
        assert isinstance(result, aquaflux.ResultFrame)
        assert isinstance(result.data, polars.DataFrame)
        expected = extra_expected[result.name]
        assert result.data.columns == expected.columns
        assert result.data.schema == expected.schema
        assert result.data.sort(expected.columns[0]).rows() == expected.rows()
    results = {result.name: result.data for result in frames}
    assert list(results) == expected_names
    assert results["C"].sort("c").equals(extra_expected["C"])
    assert extra_compiled.output_refs == extra_default_refs

try:
    extra_compiled.execute([extra_source], optional_extra_outputs=["unknown_extra"])
except RuntimeError as error:
    assert "unknown_extra" in str(error)
else:
    raise AssertionError("Expected RuntimeError for unknown_extra")
assert extra_compiled.output_refs == extra_default_refs
no_extra_frames = extra_compiled.execute([extra_source])
assert [result.name for result in no_extra_frames] == ["C"]
assert no_extra_frames[0].data.sort("c").equals(extra_expected["C"])
print("Extra output contract assertions passed")


def assert_graph_error(action, error_type, *message_parts):
    # Match the public error type and stable context, not full Polars diagnostics.
    try:
        action()
    except error_type as error:
        for part in message_parts:
            assert part in str(error), f"Missing {part!r} in {error!r}"
    else:
        raise AssertionError(f"Expected {error_type.__name__}")


for attribute in ("input_refs", "output_refs"):
    assert_graph_error(lambda: setattr(compiled_graph, attribute, []), AttributeError)

named_orders = aquaflux.NamedFrame(polars.DataFrame(graph_data), "orders")
unexpected_orders = aquaflux.NamedFrame(polars.DataFrame(graph_data), "unexpected")
assert_graph_error(
    lambda: compiled_graph.execute([]), RuntimeError, "Missing input 'orders'"
)
assert_graph_error(
    lambda: compiled_graph.execute([named_orders, named_orders]),
    RuntimeError, "Duplicate input 'orders'",
)
assert_graph_error(
    lambda: compiled_graph.execute([named_orders, unexpected_orders]),
    ValueError, "Unexpected input 'unexpected'",
)
assert_graph_error(lambda: aquaflux.NamedFrame(object(), "orders"), TypeError, "DataFrame")
assert_graph_error(lambda: compiled_graph.execute([object()]), TypeError, "NamedFrame")

# Missing references are rejected by compile(), not by graph construction.
for missing_ref in ("input_ref", "output_ref"):
    incomplete = aquaflux.Section([aquaflux.SelectOp(["amount"])])
    incomplete.name = "incomplete"
    if missing_ref != "input_ref":
        incomplete.input_ref = "orders"
    if missing_ref != "output_ref":
        incomplete.output_ref = "result"
    incomplete_graph = aquaflux.ExecutionGraph([incomplete])
    assert_graph_error(
        incomplete_graph.compile, ValueError, "Section 0", "no", missing_ref.removesuffix("_ref")
    )

# Duplicate producers and cycles may be rejected during graph construction.
duplicate = aquaflux.Section([aquaflux.SelectOp(["amount"])])
duplicate.name = "duplicate_producer"
duplicate.input_ref = "orders"
duplicate.output_ref = "cleaned"
assert_graph_error(
    lambda: aquaflux.ExecutionGraph([clean_orders, duplicate]).compile(),
    ValueError, "cleaned", "duplicate producer section 1", "duplicate_producer",
)

cycle = aquaflux.Section([aquaflux.SelectOp(["amount"])])
cycle.name = "cycle"
cycle.input_ref = "cleaned"
cycle.output_ref = "orders"
assert_graph_error(
    lambda: aquaflux.ExecutionGraph([clean_orders, cycle]).compile(),
    ValueError, "Cycle detected", "sections", "orders", "cleaned",
)

# Invalid operation lowering reports original section and operation positions.
bad_lowering = aquaflux.Section([
    aquaflux.SelectOp(["id"]),
    aquaflux.JoinOp(left_on=["id"], right_on=["id"], other="customers", how="invalid"),
])
bad_lowering.name = "bad_lowering"
bad_lowering.input_ref = "orders"
bad_lowering.output_ref = "bad_result"
bad_lowering.secondary_input_refs = ["customers"]
assert_graph_error(
    lambda: aquaflux.ExecutionGraph([clean_orders, bad_lowering]),
    ValueError, "Section 1", "bad_lowering", "operation 1", "Unknown join type", "invalid",
)
bad_lowering.instructions = [
    aquaflux.SelectOp(["id"]),
    aquaflux.JoinOp(left_on=["id"], right_on=["id"], other="customers", how="inner"),
]
bad_lowering.secondary_input_refs = []
assert_graph_error(
    lambda: aquaflux.ExecutionGraph([clean_orders, bad_lowering]),
    ValueError, "Section 1", "bad_lowering", "operation 1", "customers", "secondary_input_refs",
)

print("ExecutionGraph assertions passed")


# Independent named inputs: input_1 -> A -> C and input_2 -> B.
# Renaming before casting/filtering also protects instruction ordering.
isolated_a = aquaflux.Section([
    aquaflux.SelectOp(["id", "value"]),
    aquaflux.RenameOp(["value"], ["amount"]),
    aquaflux.CastOp(["amount"], float),
])
isolated_a.input_ref = "input_1"
isolated_a.output_ref = "A"
isolated_b = aquaflux.Section([
    aquaflux.SelectOp(["label", "count"]),
    aquaflux.RenameOp(["count"], ["b_count"]),
])
isolated_b.input_ref = "input_2"
isolated_b.output_ref = "B"
isolated_c = aquaflux.Section([
    aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 20.0),
])
isolated_c.input_ref = "A"
isolated_c.output_ref = "C"
isolated_graph = aquaflux.ExecutionGraph([isolated_c, isolated_b, isolated_a]).compile()
assert set(isolated_graph.input_refs) == {"input_1", "input_2"}
assert isolated_graph.output_refs == ["C", "B"]


def isolation_inputs(values, counts):
    return [
        aquaflux.NamedFrame(polars.DataFrame({
            "id": [2, 1], "value": values, "unused": [0, 0],
        }), "input_1"),
        aquaflux.NamedFrame(polars.DataFrame({
            "label": ["y", "x"], "count": counts,
        }), "input_2"),
    ]


def assert_isolated_run(inputs, c_rows, b_rows):
    frames = isolated_graph.execute(inputs)
    assert [result.name for result in frames] == ["C", "B"]
    assert all(isinstance(result, aquaflux.ResultFrame) for result in frames)
    results = {result.name: result.data for result in frames}
    assert all(isinstance(data, polars.DataFrame) for data in results.values())
    assert results["C"].columns == ["id", "amount"]
    assert results["C"].schema == {"id": polars.Int64, "amount": polars.Float64}
    assert results["C"].sort("id").rows() == c_rows
    assert results["B"].columns == ["label", "b_count"]
    assert results["B"].schema == {"label": polars.String, "b_count": polars.Int64}
    assert results["B"].sort("label").rows() == b_rows
    assert isolated_graph.output_refs == ["C", "B"]
    return results


first_inputs = isolation_inputs(["10", "30"], [7, 3])
second_inputs = isolation_inputs(["90", "40"], [100, 200])
first_run = assert_isolated_run(first_inputs, [(1, 30.0)], [("x", 3), ("y", 7)])
assert_isolated_run(second_inputs, [(1, 40.0), (2, 90.0)], [("x", 200), ("y", 100)])
# Requesting A does not remove the independent B branch or its required input.
assert_graph_error(
    lambda: isolated_graph.execute(first_inputs[:1], optional_extra_outputs=["A"]),
    RuntimeError, "Missing input 'input_2'",
)
assert_isolated_run(first_inputs, [(1, 30.0)], [("x", 3), ("y", 7)])

# Lazy-plan collection errors remain RuntimeError with actionable column context.
for invalid_frame, context in (
    (polars.DataFrame({"id": [1], "wrong_column": [10]}), "value"),
    (polars.DataFrame({"id": [1], "value": [[1, 2]]}), "cast"),
):
    invalid_inputs = [aquaflux.NamedFrame(invalid_frame, "input_1"), second_inputs[1]]
    assert_graph_error(
        lambda: isolated_graph.execute(invalid_inputs),
        RuntimeError, "Failed to collect output plan", context,
    )
    # Section plans are built before unknown extra references are resolved;
    # they are not collected when output resolution fails.
    assert_graph_error(
        lambda: isolated_graph.execute(invalid_inputs, optional_extra_outputs=["unknown_extra"]),
        RuntimeError, "Unknown reference 'unknown_extra'",
    )
    assert_isolated_run(second_inputs, [(1, 40.0), (2, 90.0)], [("x", 200), ("y", 100)])
assert first_run["C"].sort("id").rows() == [(1, 30.0)]
assert first_run["B"].sort("label").rows() == [("x", 3), ("y", 7)]
print("Independent-input run isolation assertions passed")


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
assert [result.name for result in example_frames] == example_graph.output_refs
assert len(example_frames) == 2
assert all(isinstance(result, aquaflux.ResultFrame) for result in example_frames)
example_results = {result.name: result.data for result in example_frames}
assert all(isinstance(data, polars.DataFrame) for data in example_results.values())
assert example_results["customer_sales"].columns == [
    "customer", "total_sales", "average_order", "items_sold",
]
assert example_results["customer_sales"].schema == {
    "customer": polars.String, "total_sales": polars.Float64,
    "average_order": polars.Float64, "items_sold": polars.Int64,
}
assert example_results["customer_sales"].rows() == [
    ("Bob", 410.0, 205.0, 7),
    ("Charlie", 120.0, 120.0, 1),
    ("Alice", 120.0, 60.0, 7),
    ("Unknown", 60.0, 60.0, 2),
]
assert example_results["high_value_orders"].columns == [
    "order_id", "customer", "quantity", "order_total",
]
assert example_results["high_value_orders"].schema == {
    "order_id": polars.Int64, "customer": polars.String,
    "quantity": polars.Int64, "order_total": polars.Float64,
}
assert example_results["high_value_orders"].rows() == [
    (5, "Bob", 5, 250.0), (2, "Bob", 2, 160.0),
]

print("\nCustomer sales leaderboard (all orders, grouped by customer):")
print(example_results["customer_sales"])
print("\nHigh-value orders (individual orders with a total above 150):")
print(example_results["high_value_orders"])


# Three inputs, two reference-based joins, and two reports from shared enrichment.
print("\n--- Example: Join Orders, Products, and Customers ---")
print(
    "products -> prepared_products --+\n"
    "                               | secondary input\n"
    "orders --------------------> enriched_orders -> regional_sales\n"
    "                               |             -> large_orders\n"
    "customers ---------------------+ secondary input"
)

multi_orders = polars.DataFrame({
    "order_id": [1, 2, 3, 4, 5],
    "customer_id": [101, 102, 101, 999, 101],
    "sku": ["A", "B", "B", "A", "A"],
    "quantity": [2, 3, 1, 4, 0],
})
multi_products = polars.DataFrame({
    "sku": ["A", "B"],
    "unit_price": ["10.0", "20.0"],
})
multi_customers = polars.DataFrame({
    "customer_id": [101, 102],
    "customer": ["Alice", "Bob"],
    "region": ["North", "South"],
})
for label, frame in (
    ("Orders", multi_orders),
    ("Products", multi_products),
    ("Customers", multi_customers),
):
    print(f"\n{label} input:")
    print(frame)

multi_prepare_products = aquaflux.Section([
    aquaflux.CastOp(["unit_price"], float),
])
multi_prepare_products.name = "prepare_product_prices"
multi_prepare_products.input_ref = "products"
multi_prepare_products.output_ref = "prepared_products"

multi_enrich = aquaflux.Section([
    aquaflux.FilterOp("quantity", aquaflux.LogicalOp.Gt, 0),
    aquaflux.JoinOp(
        left_on=["sku"],
        right_on=["sku"],
        other="prepared_products",
        how="inner",
    ),
    aquaflux.JoinOp(
        left_on=["customer_id"],
        right_on=["customer_id"],
        other="customers",
        how="left",
    ),
    aquaflux.FillNaOp(["customer", "region"], "Unknown"),
    aquaflux.WithColumns([
        (aquaflux.Col("unit_price") * aquaflux.Col("quantity")).alias("order_total"),
    ]),
])
multi_enrich.name = "join_and_price_orders"
multi_enrich.input_ref = "orders"
# One secondary plan is produced internally; the other is an external input.
multi_enrich.secondary_input_refs = ["prepared_products", "customers"]
multi_enrich.output_ref = "enriched_orders"

multi_summary = aquaflux.Section([
    aquaflux.GroupByOp(
        group_columns=["region"],
        aggregations=[
            ("order_total", aquaflux.AggOp.Sum, "total_sales"),
            ("quantity", aquaflux.AggOp.Sum, "items_sold"),
        ],
    ),
    aquaflux.SortOp(["region"], descending=False),
])
multi_summary.name = "report_regional_sales"
multi_summary.input_ref = "enriched_orders"
multi_summary.output_ref = "regional_sales"

multi_large_orders = aquaflux.Section([
    aquaflux.FilterOp("order_total", aquaflux.LogicalOp.Gt, 30.0),
    aquaflux.SelectOp(["order_id", "customer", "region", "order_total"]),
    aquaflux.SortOp(["order_total", "order_id"], descending=True),
])
multi_large_orders.name = "report_large_orders"
multi_large_orders.input_ref = "enriched_orders"
multi_large_orders.output_ref = "large_orders"

# Reports precede enrichment, which itself precedes its secondary-plan producer.
multi_graph = aquaflux.ExecutionGraph([
    multi_summary,
    multi_large_orders,
    multi_enrich,
    multi_prepare_products,
]).compile()
assert set(multi_graph.input_refs) == {"orders", "products", "customers"}
assert set(multi_graph.output_refs) == {"regional_sales", "large_orders"}

# Named inputs can be supplied in any order; no intermediate frames are required.
multi_frames = multi_graph.execute([
    aquaflux.NamedFrame(multi_customers, "customers"),
    aquaflux.NamedFrame(multi_products, "products"),
    aquaflux.NamedFrame(multi_orders, "orders"),
])
assert len(multi_frames) == 2
assert [result.name for result in multi_frames] == multi_graph.output_refs
assert all(isinstance(result, aquaflux.ResultFrame) for result in multi_frames)
multi_results = {result.name: result.data for result in multi_frames}
assert all(isinstance(data, polars.DataFrame) for data in multi_results.values())
assert multi_results["regional_sales"].schema == {
    "region": polars.String, "total_sales": polars.Float64, "items_sold": polars.Int64,
}
assert multi_results["large_orders"].schema == {
    "order_id": polars.Int64, "customer": polars.String,
    "region": polars.String, "order_total": polars.Float64,
}
assert multi_results["regional_sales"].columns == ["region", "total_sales", "items_sold"]
assert multi_results["regional_sales"].rows() == [
    ("North", 40.0, 3),
    ("South", 60.0, 3),
    ("Unknown", 40.0, 4),
]
assert multi_results["large_orders"].columns == [
    "order_id", "customer", "region", "order_total",
]
assert multi_results["large_orders"].rows() == [
    (2, "Bob", "South", 60.0),
    (4, "Unknown", "Unknown", 40.0),
]

print("\nRegional sales (zero-quantity orders excluded):")
print(multi_results["regional_sales"])
print("\nLarge orders (unmatched customers retained by the left join):")
print(multi_results["large_orders"])
print("Multi-input join graph assertions passed")
