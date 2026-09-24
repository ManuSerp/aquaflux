use crate::CompiledSection;
use crate::graph::reference::{FramedReference, ReferenceTable};
use crate::interface::section::PySection;
use crate::pipeline::{IntoLazy, LazyExecutable, dataframe::from_python, dataframe::to_python};
use polars::lazy::frame::LazyFrame;
use pyo3::prelude::*;
pub mod reference;
pub mod section;

// TODO Next branch steps (see DAG_ARCHITECTURE.md):
// 1. Build an ExecutionGraph compiler around the existing sections, accepting
//    multiple sections and requested output references. Decide external input
//    declaration rules so unknown references/typos can be rejected explicitly.
// 2. Build a reference -> producer table from external inputs and section outputs;
//    reject duplicate producers, input/output collisions, and missing outputs.
// 3. Track primary and auxiliary dependencies (including reference-based
//    JoinOp.other); define how operations resolve auxiliary LazyFrames while
//    keeping ordinary unary operations simple.
// 4. Resolve dependencies across all sections, allowing forward references;
//    detect cycles and compute a topological order without reordering operations
//    inside a section. Include section/reference names in validation errors.
// 5. Expose the compiled input/output contract and logical graph for inspection
//    (sections, producers, consumers, edges, execution order). If pruning is
//    supported, require only inputs needed for the requested outputs.
// 6. Add a per-execution reference store: validate/convert named inputs once,
//    resolve and reuse lazy plans, and register section outputs in graph order.
//    Define LazyFrame cloning/lifetimes; do not collect between sections or turn
//    reference reuse into implicit eager caching.
// 7. Collect only requested outputs and return them by reference name. Investigate
//    Polars multi-output collection/shared-branch recomputation before choosing
//    an execution strategy.
// 8. Preserve compile_pipeline([ops]).execute(data) via one implicit section and
//    input/output pair; keep eager JoinOp.other temporarily compatible.
// 9. Add Rust/Python assertion tests for multiple inputs/outputs, branch reuse,
//    reference joins, out-of-order declarations, invalid graphs, and linear API
//    compatibility. Leave loaders, advanced caching, and dynamic outputs for later.

pub struct IndexedSection {
    pub index: usize,
    pub section: CompiledSection,
}

pub struct ExecutionGraph {
    pub sections: Vec<IndexedSection>,
    pub execution_layers: Vec<Vec<usize>>,
    /// Producer-to-consumer adjacency list, indexed by original section index.
    pub dependency_tree: Vec<Vec<usize>>,
    pub ref_table: ReferenceTable,
}

impl ExecutionGraph {
    pub fn new(sections: Vec<PySection>) -> Result<Self, String> {
        // build indexed sections
        let isections: Vec<IndexedSection> = sections
            .into_iter()
            .enumerate()
            .map(|(i, section)| {
                section
                    .compile()
                    .map(|section| IndexedSection { index: i, section })
                    .map_err(|err| err.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        // reference table:
        let mut table = ReferenceTable::new();
        table.build(&isections)?;
        let (layers, tree) = table.build_execution_layers(isections.len())?;
        Ok(Self {
            sections: isections,
            execution_layers: layers,
            dependency_tree: tree,
            ref_table: table,
        })
    }
}

pub struct CompiledExecutionGraph {
    pub graph: ExecutionGraph,
    pub input_refs: Vec<String>,
    pub output_refs: Vec<String>,
    //the lazy plan object nned to be a collection of Lazyframe ? probably one per output ? on zhich ze zill call collect all ?
    // input will be converted as lazy frame when exec called on them then we recursively apply the section plan on them and the call collect that is the goal
    // so the compilation can probaly be converting the section with PySection to Compiled section: But could we pre built lazy plan for intermediate step so execute only apply to input and bam done ?
}

impl CompiledExecutionGraph {
    pub fn apply_plan(&self, inputs: Vec<FramedReference>) -> Result<Vec<FramedReference>, String> {
        let table = &self.graph.ref_table;
        // Slots share the reference table's indices and retain plans for branching.
        let mut plans: Vec<Option<LazyFrame>> = vec![None; table.references.len()];

        let reference_id = |name: &str| -> Result<usize, String> {
            table
                .ref_map
                .get(name)
                .copied()
                .ok_or_else(|| format!("Unknown reference '{name}'"))
        };
        // load input plans into the plan slots, and check that all required inputs are present

        for input in inputs {
            let name = &input.reference.name;
            let id = reference_id(name)?;

            if !self.input_refs.contains(name) {
                return Err(format!("Unexpected input '{name}'"));
            }
            if !table.references[id].needs.is_empty() {
                return Err(format!("Input '{name}' is produced by a section"));
            }
            if plans[id].is_some() {
                return Err(format!("Duplicate input '{name}'"));
            }

            plans[id] = Some(input.lazyframe);
        }
        for name in &self.input_refs {
            let id = reference_id(name)?;
            if plans[id].is_none() {
                return Err(format!("Missing input '{name}'"));
            }
        }
        // execute the sections in topological order, building plans for each output reference
        // TODO adapt the logic to support multiple inputs
        for layer in &self.graph.execution_layers {
            for &section_index in layer {
                let section = &self.graph.sections[section_index].section;
                let input_name = section.input_ref.as_deref().ok_or_else(|| {
                    format!("Section {section_index} has no input reference")
                    // TODO we probably should support that
                })?;
                let output_name = section
                    .output_ref
                    .as_deref()
                    .ok_or_else(|| format!("Section {section_index} has no output reference"))?;

                let input_id = reference_id(input_name)?;
                let output_id = reference_id(output_name)?;
                let mut plan = plans[input_id]
                    .as_ref()
                    .ok_or_else(|| {
                        format!("Section {section_index}: input '{input_name}' is not available")
                    })?
                    .clone();

                if plans[output_id].is_some() {
                    return Err(format!(
                        "Section {section_index}: output '{output_name}' already exists"
                    ));
                }

                for (op_index, op) in section.instructions.iter().enumerate() {
                    plan = op.execute_lazy(plan).map_err(|err| {
                        format!("Section {section_index}, operation {op_index}: {err}")
                    })?;
                }

                plans[output_id] = Some(plan);
            }
        }
        // TODO use the fact that we have the plan for intermediate value for debug mod
        self.output_refs
            .iter()
            .map(|name| {
                let id = reference_id(name)?;
                let lazyframe = plans[id]
                    .as_ref()
                    .ok_or_else(|| format!("Output '{name}' was not built"))?
                    .clone();

                Ok(FramedReference {
                    reference: table.references[id].clone(),
                    lazyframe,
                })
            })
            .collect()
    }
    pub fn execute<'py>(
        &self,
        py: Python<'py>,
        input_data: Vec<NamedFrame>, //Can this be variadic ?
    ) -> PyResult<Vec<Bound<'py, PyAny>>> {
        // first created NamedReference from named frame
        // we also need a check that ref table was builded (need to be the case to get that struct)
        let mut input_refs: Vec<FramedReference> = Vec::new();
        for nframe in input_data.iter() {
            if !self.input_refs.contains(&nframe.name) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unexpected input '{name}'",
                    name = nframe.name
                )));
            } else {
                let fref = self
                    .graph
                    .ref_table
                    .ref_map
                    .get(&nframe.name)
                    .ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Input reference '{name}' not found in graph ref table",
                            name = nframe.name
                        ))
                    })?;
                input_refs.push(FramedReference {
                    reference: self.graph.ref_table.references[*fref].clone(),
                    lazyframe: nframe.data.clone(),
                });
            }
        }
        // build plan
        let output_plan = self.apply_plan(input_refs).map_err(|err| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to apply execution plan: {err}"
            ))
        })?;
        // resolve the plan with polars and materialize output
        let resolved = LazyFrame::collect_all_with_engine(
            output_plan
                .into_iter()
                .map(|fr| fr.lazyframe.logical_plan)
                .collect(),
            polars::prelude::Engine::Auto,
            polars::prelude::OptFlags::default(),
        )
        .map_err(|err| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to collect output plan: {err}"
            ))
        })?;

        resolved
            .into_iter()
            .map(|df| to_python(py, df))
            .collect::<PyResult<Vec<_>>>()
    }
}

// this will probably go to interface
pub struct NamedFrame {
    data: LazyFrame,
    name: String,
}

impl NamedFrame {
    pub fn new_from_lf(data: LazyFrame, name: String) -> Self {
        Self { data, name }
    }

    pub fn new_from_py<'py>(py: Python<'py>, data: &Bound<'py, PyAny>, name: String) -> Self {
        let df = from_python(data).unwrap_or_else(|err| {
            panic!("Failed to convert Python object to LazyFrame: {err}");
        });
        Self {
            data: df.lazy(),
            name,
        }
    }
}
