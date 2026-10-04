use crate::CompiledSection;
use crate::interface::PyOp;
use crate::pipeline::{self, LazyExecutable};
use pyo3::exceptions::PyValueError;
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
    pub secondary_input_refs: Option<Vec<String>>,
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
            secondary_input_refs: None,
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
        let secondary_input_refs = self.secondary_input_refs.as_deref().unwrap_or(&[]);
        let mut missing_refs = Vec::new();
        for op in &compiled {
            let (flag, needs) = op.is_secondary_input_ref_needed();
            if flag {
                let needs = needs.ok_or_else(|| {
                    PyValueError::new_err("op requires a secondary input ref but none was provided")
                })?;
                for needed_ref in needs {
                    if !secondary_input_refs.contains(&needed_ref)
                        && !missing_refs.contains(&needed_ref)
                    {
                        missing_refs.push(needed_ref);
                    }
                }
            }
        }
        if !missing_refs.is_empty() {
            return Err(PyValueError::new_err(format!(
                "secondary_input_refs does not contain needed refs: {}",
                missing_refs.join(", ")
            )));
        }
        Ok(CompiledSection {
            instructions: compiled,
            name: self.name.clone(),
            input_ref: self.input_ref.clone(),
            secondary_input_refs: self.secondary_input_refs.clone(),
            output_ref: self.output_ref.clone(),
        })
    }
}
