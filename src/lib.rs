pub mod pipeline;

use pyo3::prelude::*;
#[pymodule]
mod aquaflux {
    use crate::pipeline::Instructions;
    use pyo3::prelude::*;

    #[pyclass]
    pub struct CompiledPipeline {
        pub instructions: Vec<Instructions>,
    }

    #[pyfunction]
    pub fn compile_pipeline(spec: &str) -> PyResult<CompiledPipeline> {
        let ops = parse_pipeline(spec);
        Ok(CompiledPipeline { instructions: ops })
    }
}
