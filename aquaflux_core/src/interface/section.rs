use crate::interface::extract_operation_type;
use pyo3::prelude::*;
#[pyclass(name = "Section", from_py_object)]
#[derive(Clone)]
pub struct PySection {
    #[pyo3(get, set)]
    pub instructions: Vec<Py<pyo3::types::PyType>>,
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
    pub fn new(ops: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let instructions = ops
            .iter()
            .map(|op| extract_operation_type(op).map(Bound::unbind))
            .collect::<PyResult<Vec<_>>>()?;
        // Vec<Pyop>
        Ok(PySection {
            instructions,
            name: None,
            input_ref: None,
            output_ref: None,
        })
    }
}
