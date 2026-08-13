#!/usr/bin/env python3
"""
Benchmark: Pandas vs Aquaflux vs Polars

This script compares performance of equivalent data transformation pipelines
across three frameworks with varying data sizes.
"""

import argparse
import time
from dataclasses import dataclass
from typing import Callable, Any, Literal
import numpy as np
import pandas as pd
import polars as pl
import aquaflux_core as aquaflux
from aquaflux_core import Col, WithColumns


# =============================================================================
# Global Configuration
# =============================================================================

# Which DataFrame type to feed to Aquaflux: "polars" or "pandas"
AQUAFLUX_INPUT_TYPE: Literal["polars", "pandas"] = "polars"


# =============================================================================
# Benchmark Infrastructure
# =============================================================================

@dataclass
class BenchmarkResult:
    """Result of a single benchmark run."""
    framework: str
    operation: str
    data_size: int
    elapsed_ms: float

    def __repr__(self) -> str:
        return f"{self.framework:12} | {self.operation:20} | {self.data_size:>10,} rows | {self.elapsed_ms:>10.2f} ms"


def time_execution(func: Callable[[], Any], warmup: int = 1, runs: int = 3) -> float:
    """Time a function execution with warmup runs. Returns median time in ms."""
    # Warmup
    for _ in range(warmup):
        func()

    # Timed runs
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        func()
        elapsed = (time.perf_counter() - start) * 1000  # Convert to ms
        times.append(elapsed)

    return np.median(times)


# =============================================================================
# Data Generation
# =============================================================================

def generate_sales_data(n_rows: int, seed: int = 42) -> pd.DataFrame:
    """Generate synthetic sales data for benchmarking."""
    np.random.seed(seed)

    categories = ["Electronics", "Clothing", "Food", "Books", "Home", "Sports"]
    regions = ["North", "South", "East", "West", "Central"]

    # Add some nulls to make it realistic
    customers = [f"Customer_{i}" if np.random.random() > 0.05 else None for i in range(n_rows)]

    return pd.DataFrame({
        "customer": customers,
        "order_id": range(1, n_rows + 1),
        "amount": np.random.uniform(10.0, 1000.0, n_rows).astype(str),  # String for casting test
        "quantity": np.random.randint(1, 20, n_rows),
        "category": np.random.choice(categories, n_rows),
        "region": np.random.choice(regions, n_rows),
        "discount": np.random.uniform(0.0, 0.3, n_rows),
        "min_threshold": np.random.uniform(100.0, 500.0, n_rows),
    })


# =============================================================================
# Benchmark 1: Basic Pipeline (Select, FillNa, Cast, Filter, Rename)
# =============================================================================

def bench_basic_pipeline_pandas(df: pd.DataFrame) -> pd.DataFrame:
    """Pandas implementation of basic pipeline."""
    result = df[["customer", "order_id", "amount", "category"]].copy()
    result["customer"] = result["customer"].fillna("Unknown")
    result["amount"] = result["amount"].astype(float)
    result = result[result["amount"] > 200.0]
    result = result.rename(columns={"customer": "customer_name"})
    result = result.dropna()
    return result


def bench_basic_pipeline_polars(df: pl.DataFrame) -> pl.DataFrame:
    """Polars implementation of basic pipeline."""
    return (
        df.select(["customer", "order_id", "amount", "category"])
        .with_columns(pl.col("customer").fill_null("Unknown"))
        .with_columns(pl.col("amount").cast(pl.Float64))
        .filter(pl.col("amount") > 200.0)
        .rename({"customer": "customer_name"})
        .drop_nulls()
    )


def create_basic_pipeline_aquaflux() -> aquaflux.CompiledPipeline:
    """Aquaflux implementation of basic pipeline."""
    return aquaflux.compile_pipeline([
        aquaflux.SelectOp(["customer", "order_id", "amount", "category"]),
        aquaflux.FillNaOp(["customer"], "Unknown"),
        aquaflux.CastOp(["amount"], float),
        aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 200.0),
        aquaflux.RenameOp(["customer"], ["customer_name"]),
        aquaflux.DropNaOp(),
    ])


# =============================================================================
# Benchmark 2: GroupBy Aggregation
# =============================================================================

def bench_groupby_pandas(df: pd.DataFrame) -> pd.DataFrame:
    """Pandas implementation of groupby."""
    return df.groupby("category").agg(
        total_amount=("amount", "sum"),
        avg_amount=("amount", "mean"),
        order_count=("order_id", "count"),
    ).reset_index()


def bench_groupby_polars(df: pl.DataFrame) -> pl.DataFrame:
    """Polars implementation of groupby."""
    return df.group_by("category").agg(
        pl.col("amount").sum().alias("total_amount"),
        pl.col("amount").mean().alias("avg_amount"),
        pl.col("order_id").count().alias("order_count"),
    )


def create_groupby_pipeline_aquaflux() -> aquaflux.CompiledPipeline:
    """Aquaflux implementation of groupby."""
    return aquaflux.compile_pipeline([
        aquaflux.GroupByOp(
            group_columns=["category"],
            aggregations=[
                ("amount", aquaflux.AggOp.Sum, "total_amount"),
                ("amount", aquaflux.AggOp.Mean, "avg_amount"),
                ("order_id", aquaflux.AggOp.Count, "order_count"),
            ],
        )
    ])


# =============================================================================
# Benchmark 3: WithColumns / Computed Columns
# =============================================================================

def bench_with_columns_pandas(df: pd.DataFrame) -> pd.DataFrame:
    """Pandas implementation of computed columns."""
    result = df.copy()
    result["total_value"] = result["amount"] * result["quantity"]
    result["amount_doubled"] = result["amount"] * 2
    result["amount_plus_qty"] = result["amount"] + result["quantity"]
    return result


def bench_with_columns_polars(df: pl.DataFrame) -> pl.DataFrame:
    """Polars implementation of computed columns."""
    return df.with_columns([
        (pl.col("amount") * pl.col("quantity")).alias("total_value"),
        (pl.col("amount") * 2).alias("amount_doubled"),
        (pl.col("amount") + pl.col("quantity")).alias("amount_plus_qty"),
    ])


def create_with_columns_pipeline_aquaflux() -> aquaflux.CompiledPipeline:
    """Aquaflux implementation of computed columns."""
    return aquaflux.compile_pipeline([
        WithColumns([
            (Col("amount") * Col("quantity")).alias("total_value"),
            (Col("amount") * 2).alias("amount_doubled"),
            (Col("amount") + Col("quantity")).alias("amount_plus_qty"),
        ])
    ])


# =============================================================================
# Benchmark 4: Complex Pipeline (Multiple Operations)
# =============================================================================

def bench_complex_pipeline_pandas(df: pd.DataFrame) -> pd.DataFrame:
    """Pandas implementation of complex pipeline."""
    result = df.copy()
    result["customer"] = result["customer"].fillna("Unknown")
    result["amount"] = result["amount"].astype(float)
    result = result[result["amount"] > 100.0]
    result["total_value"] = result["amount"] * result["quantity"]
    result = result[["customer", "category", "region", "amount", "total_value"]]
    result = result.groupby(["category", "region"]).agg(
        total_sales=("total_value", "sum"),
        avg_amount=("amount", "mean"),
        customer_count=("customer", "count"),
    ).reset_index()
    return result


def bench_complex_pipeline_polars(df: pl.DataFrame) -> pl.DataFrame:
    """Polars implementation of complex pipeline."""
    return (
        df
        .with_columns(pl.col("customer").fill_null("Unknown"))
        .with_columns(pl.col("amount").cast(pl.Float64))
        .filter(pl.col("amount") > 100.0)
        .with_columns((pl.col("amount") * pl.col("quantity")).alias("total_value"))
        .select(["customer", "category", "region", "amount", "total_value"])
        .group_by(["category", "region"]).agg(
            pl.col("total_value").sum().alias("total_sales"),
            pl.col("amount").mean().alias("avg_amount"),
            pl.col("customer").count().alias("customer_count"),
        )
    )


def create_complex_pipeline_aquaflux() -> aquaflux.CompiledPipeline:
    """Aquaflux implementation of complex pipeline."""
    return aquaflux.compile_pipeline([
        aquaflux.FillNaOp(["customer"], "Unknown"),
        aquaflux.CastOp(["amount"], float),
        aquaflux.FilterOp("amount", aquaflux.LogicalOp.Gt, 100.0),
        WithColumns([
            (Col("amount") * Col("quantity")).alias("total_value"),
        ]),
        aquaflux.SelectOp(["customer", "category", "region", "amount", "total_value"]),
        aquaflux.GroupByOp(
            group_columns=["category", "region"],
            aggregations=[
                ("total_value", aquaflux.AggOp.Sum, "total_sales"),
                ("amount", aquaflux.AggOp.Mean, "avg_amount"),
                ("customer", aquaflux.AggOp.Count, "customer_count"),
            ],
        ),
    ])


# =============================================================================
# Benchmark Runner
# =============================================================================

def run_benchmark(
    name: str,
    data_sizes: list[int],
    pandas_func: Callable[[pd.DataFrame], pd.DataFrame],
    polars_func: Callable[[pl.DataFrame], pl.DataFrame],
    aquaflux_pipeline: aquaflux.CompiledPipeline,
    prepare_pandas: Callable[[pd.DataFrame], pd.DataFrame] | None = None,
) -> list[BenchmarkResult]:
    """Run a benchmark across all frameworks and data sizes."""
    results = []
    
    for size in data_sizes:
        print(f"\n  Running {name} with {size:,} rows...")
        
        # Generate data
        df_pandas = generate_sales_data(size)
        
        # Prepare data if needed (e.g., cast amount to float for groupby)
        if prepare_pandas:
            df_pandas = prepare_pandas(df_pandas)
        
        df_polars = pl.from_pandas(df_pandas)
        
        # Benchmark Pandas
        elapsed = time_execution(lambda: pandas_func(df_pandas))
        results.append(BenchmarkResult("Pandas", name, size, elapsed))
        
        # Benchmark Polars
        elapsed = time_execution(lambda: polars_func(df_polars))
        results.append(BenchmarkResult("Polars", name, size, elapsed))
        
        # Benchmark Aquaflux (input type controlled by global AQUAFLUX_INPUT_TYPE)
        df_aquaflux = df_polars if AQUAFLUX_INPUT_TYPE == "polars" else df_pandas
        elapsed = time_execution(lambda: aquaflux_pipeline.execute(df_aquaflux))
        results.append(BenchmarkResult("Aquaflux", name, size, elapsed))
    
    return results


def print_results_table(results: list[BenchmarkResult]) -> None:
    """Print results in a formatted table."""
    operations = sorted(set(r.operation for r in results))
    sizes = sorted(set(r.data_size for r in results))
    frameworks = ["Pandas", "Polars", "Aquaflux"]

    for operation in operations:
        print(f"\n{'='*80}")
        print(f"Operation: {operation}")
        print(f"{'='*80}")

        header = f"{'Data Size':>15}"
        for fw in frameworks:
            header += f" | {fw:>12}"
        header += " | Fastest"
        print(header)
        print("-" * 80)

        for size in sizes:
            row_results = {
                r.framework: r.elapsed_ms
                for r in results
                if r.operation == operation and r.data_size == size
            }

            if not row_results:
                continue

            row = f"{size:>15,}"
            times = []
            for fw in frameworks:
                t = row_results.get(fw, float('nan'))
                row += f" | {t:>10.2f}ms"
                times.append((fw, t))

            fastest = min(times, key=lambda x: x[1])
            row += f" | {fastest[0]}"

            print(row)


def main():
    """Run all benchmarks."""
    global AQUAFLUX_INPUT_TYPE
    
    parser = argparse.ArgumentParser(description="Benchmark Pandas vs Aquaflux vs Polars")
    parser.add_argument(
        "--aquaflux-input",
        choices=["polars", "pandas"],
        default="polars",
        help="DataFrame type to feed to Aquaflux (default: polars)",
    )
    args = parser.parse_args()
    
    AQUAFLUX_INPUT_TYPE = args.aquaflux_input
    
    print("=" * 80)
    print("Benchmark: Pandas vs Aquaflux vs Polars")
    print(f"Aquaflux input type: {AQUAFLUX_INPUT_TYPE}")
    print("=" * 80)

    data_sizes = [1_000, 10_000, 100_000, 1_000_000]

    all_results = []

    # Benchmark 1: Basic Pipeline
    print("\n[1/4] Basic Pipeline (Select, FillNa, Cast, Filter, Rename, DropNa)")
    pipeline = create_basic_pipeline_aquaflux()
    results = run_benchmark(
        "Basic Pipeline",
        data_sizes,
        bench_basic_pipeline_pandas,
        bench_basic_pipeline_polars,
        pipeline,
    )
    all_results.extend(results)

    # Benchmark 2: GroupBy
    print("\n[2/4] GroupBy Aggregation")

    def prepare_for_groupby(df: pd.DataFrame) -> pd.DataFrame:
        df = df.copy()
        df["amount"] = df["amount"].astype(float)
        return df

    pipeline = create_groupby_pipeline_aquaflux()
    results = run_benchmark(
        "GroupBy",
        data_sizes,
        bench_groupby_pandas,
        bench_groupby_polars,
        pipeline,
        prepare_pandas=prepare_for_groupby,
    )
    all_results.extend(results)

    # Benchmark 3: WithColumns
    print("\n[3/4] WithColumns (Computed Columns)")

    def prepare_for_with_columns(df: pd.DataFrame) -> pd.DataFrame:
        df = df.copy()
        df["amount"] = df["amount"].astype(float)
        return df

    pipeline = create_with_columns_pipeline_aquaflux()
    results = run_benchmark(
        "WithColumns",
        data_sizes,
        bench_with_columns_pandas,
        bench_with_columns_polars,
        pipeline,
        prepare_pandas=prepare_for_with_columns,
    )
    all_results.extend(results)

    # Benchmark 4: Complex Pipeline
    print("\n[4/4] Complex Pipeline (FillNa, Cast, Filter, WithColumns, Select, GroupBy)")
    pipeline = create_complex_pipeline_aquaflux()
    results = run_benchmark(
        "Complex Pipeline",
        data_sizes,
        bench_complex_pipeline_pandas,
        bench_complex_pipeline_polars,
        pipeline,
    )
    all_results.extend(results)

    # Print summary
    print("\n")
    print("=" * 80)
    print("RESULTS SUMMARY")
    print("=" * 80)
    print_results_table(all_results)

    # Print detailed results
    print("\n")
    print("=" * 80)
    print("DETAILED RESULTS")
    print("=" * 80)
    for r in all_results:
        print(r)


if __name__ == "__main__":
    main()
