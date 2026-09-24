use crate::graph::{CompiledExecutionGraph, ExecutionGraph};
use crate::interface::section::PySection;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Arc;

/// A validated graph shared with compiled executions without consuming it.
#[pyclass(name = "ExecutionGraph")]
pub struct PyExecutionGraph {
    graph: Arc<ExecutionGraph>,
}

#[pymethods]
impl PyExecutionGraph {
    #[new]
    pub fn new(sections: Vec<PySection>) -> PyResult<Self> {
        let graph = ExecutionGraph::new(sections).map_err(PyValueError::new_err)?;
        Ok(Self {
            graph: Arc::new(graph),
        })
    }

    /// Compile with all external inputs and terminal outputs inferred from sections.
    pub fn compile(&self) -> PyResult<CompiledExecutionGraph> {
        self.graph.compile().map_err(PyValueError::new_err)
    }
}
