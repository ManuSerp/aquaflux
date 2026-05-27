pub mod interface;
pub mod pipeline;

use crate::pipeline::Executable;
use pyo3::prelude::*;
#[pymodule]
fn aquaflux_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register the compile_pipeline function
    m.add_function(wrap_pyfunction!(compile_pipeline, m)?)?;

    // Register Python-facing types
    m.add_class::<interface::PySelectOp>()?;
    m.add_class::<interface::PyFillNaOp>()?;
    m.add_class::<interface::PyCastOp>()?;
    m.add_class::<interface::PyRenameOp>()?;
    m.add_class::<interface::PyDataType>()?;
    m.add_class::<interface::PyScalarValue>()?;
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
        let mut df = pipeline::dataframe::from_python(data)?;

        // Execute operations
        for op in &self.instructions {
            df = op
                .execute(df)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;
        }

        pipeline::dataframe::to_python(py, df)
    }
}

#[pyfunction]
pub fn compile_pipeline(_py: Python, ops: Vec<Bound<'_, PyAny>>) -> PyResult<CompiledPipeline> {
    let mut instructions = Vec::new();

    for op in ops {
        // Try to extract each operation type
        if let Ok(select_op) = op.extract::<interface::PySelectOp>() {
            instructions.push(pipeline::Op::Select(select_op.into()));
        } else if let Ok(fillna_op) = op.extract::<interface::PyFillNaOp>() {
            instructions.push(pipeline::Op::FillNa(fillna_op.into()));
        } else if let Ok(cast_op) = op.extract::<interface::PyCastOp>() {
            instructions.push(pipeline::Op::Cast(cast_op.into()));
        } else if let Ok(rename_op) = op.extract::<interface::PyRenameOp>() {
            instructions.push(pipeline::Op::Rename(rename_op.into()));
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Expected operation type, got: {}",
                op.get_type().name()?
            )));
        }
    }

    Ok(CompiledPipeline { instructions })
}
