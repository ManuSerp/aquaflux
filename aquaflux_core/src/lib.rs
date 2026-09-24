pub mod compiler;
pub mod graph;
pub mod interface;
pub mod pipeline;
pub use crate::graph::section::{CompiledSection, compile_pipeline};
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
    m.add_class::<interface::section::PySection>()?;
    m.add_class::<CompiledSection>()?;
    m.add_class::<interface::graph::PyExecutionGraph>()?;
    m.add_class::<graph::CompiledExecutionGraph>()?;
    m.add_class::<graph::NamedFrame>()?;

    Ok(())
}
