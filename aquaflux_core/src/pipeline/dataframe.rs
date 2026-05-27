use polars::prelude::DataFrame as PolarsDataFrame;
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;

pub type DataFrame = PolarsDataFrame;

/// Convert Python DataFrame to Rust Polars DataFrame

pub fn from_python(obj: &Bound<'_, PyAny>) -> PyResult<DataFrame> {
    let py = obj.py();

    // Check if it's already a Polars DataFrame
    if let Ok(pydf) = obj.extract::<pyo3_polars::PyDataFrame>() {
        return Ok(pydf.0);
    }

    // If not, try to convert from pandas
    // Import polars module
    // Warn the user that converting from pandas is less efficient
    py.import("warnings")?
        .call_method1("warn", (
            "Converting from pandas DataFrame. For better performance, pass a Polars DataFrame directly.",
        ))?;

    let polars = py.import("polars")?;

    // Call polars.from_pandas(obj)
    let polars_df = polars.call_method1("from_pandas", (obj,))?;

    // Now extract as PyDataFrame
    let pydf: pyo3_polars::PyDataFrame = polars_df.extract()?;
    Ok(pydf.0)
}

/// Convert Rust Polars DataFrame to Python Polars DataFrame
pub fn to_python(py: Python<'_>, df: DataFrame) -> PyResult<Bound<'_, PyAny>> {
    // pyo3_polars::PyDataFrame implements IntoPy<PyObject>
    let pydf = pyo3_polars::PyDataFrame(df);
    Ok(pydf.into_pyobject(py)?)
}
