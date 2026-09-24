use crate::CompiledSection;
use crate::interface::PyOp;
use crate::pipeline;
use pyo3::prelude::*;
#[pyclass(name = "Section", from_py_object)]
#[derive(Clone)]
pub struct PySection {
    #[pyo3(get, set)]
    pub instructions: Vec<PyOp>,
    #[pyo3(get, set)]
    pub name: Option<String>,
    #[pyo3(get, set)]
    pub input_ref: Option<String>,
    #[pyo3(get, set)]
    pub output_ref: Option<String>,
}

#[pymethods]
impl PySection {
    #[new]
    pub fn new(ops: Vec<PyOp>) -> Self {
        Self {
            instructions: ops,
            name: None,
            input_ref: None,
            output_ref: None,
        }
    }

    pub fn compile(&self) -> PyResult<CompiledSection> {
        let compiled = self
            .instructions
            .iter()
            .cloned()
            .map(pipeline::Op::try_from)
            .collect::<PyResult<Vec<_>>>()?;

        Ok(CompiledSection {
            instructions: compiled,
            name: self.name.clone(),
            input_ref: self.input_ref.clone(),
            output_ref: self.output_ref.clone(),
        })
    }
}
