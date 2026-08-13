pub mod compiler;
pub mod interface;
pub mod pipeline;
use crate::pipeline::{IntoLazy, LazyExecutable};
use pyo3::prelude::*;
#[pymodule]
fn aquaflux_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register the compile_pipeline function
    m.add_function(wrap_pyfunction!(compile_pipeline, m)?)?;

    // Register all operation types using the generated list
    for register_fn in interface::ALL_OPERATION_TYPES {
        register_fn(m)?;
    }

    // Register helper types
    m.add_class::<interface::PyDataType>()?;
    m.add_class::<interface::PyScalarValue>()?;
    m.add_class::<interface::PyLogicalOperator>()?;
    m.add_class::<interface::PyAggregationFunc>()?;
    m.add_class::<interface::PyMut>()?;
    m.add_class::<interface::PyCol>()?;
    m.add_class::<CompiledPipeline>()?;

    Ok(())
}

#[pyclass]
pub struct CompiledPipeline {
    pub instructions: Vec<pipeline::Op>,
}

#[pymethods]
impl CompiledPipeline {
    pub fn __repr__(&self) -> String {
        format!("CompiledPipeline({} operations)", self.instructions.len())
    }

    pub fn execute<'py>(
        &self,
        py: Python<'py>,
        data: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let df = pipeline::dataframe::from_python(data)?;

        // Convert to lazy once at the start
        let mut lf = df.lazy();

        // Execute all operations on the LazyFrame
        for op in &self.instructions {
            lf = op
                .execute_lazy(lf)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        }

        // Collect only once at the end
        let result = lf.collect().map_err(|e: polars::prelude::PolarsError| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
        })?;

        pipeline::dataframe::to_python(py, result)
    }
}

#[pyfunction]
pub fn compile_pipeline(_py: Python, ops: Vec<Bound<'_, PyAny>>) -> PyResult<CompiledPipeline> {
    let mut instructions = Vec::new();

    for op in ops {
        instructions.push(interface::extract_operation(&op)?);
    }

    Ok(CompiledPipeline { instructions })
}
