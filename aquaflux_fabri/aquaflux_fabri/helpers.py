"""Helper functions for building aquaflux pipelines."""
import aquaflux_core as aquaflux


def create_preprocessing_pipeline(numeric_cols, categorical_cols):
    """Create a standard preprocessing pipeline.

    Args:
        numeric_cols: List of numeric column names
        categorical_cols: List of categorical column names

    Returns:
        Compiled aquaflux pipeline
    """
    operations = []

    if numeric_cols:
        operations.append(aquaflux.FillNaOp(numeric_cols, 0.0))
        operations.append(aquaflux.CastOp(numeric_cols, float))

    if categorical_cols:
        operations.append(aquaflux.FillNaOp(categorical_cols, "Unknown"))

    return aquaflux.compile_pipeline(operations)
