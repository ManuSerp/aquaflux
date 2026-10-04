use crate::CompiledSection;
use crate::graph::reference::{FramedReference, ReferenceTable};
use crate::interface::section::PySection;
use crate::pipeline::{IntoLazy, LazyExecutable, dataframe::from_python, dataframe::to_python};
use polars::lazy::frame::LazyFrame;
use pyo3::prelude::*;
use std::sync::Arc;
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

    pub fn compile(self: &Arc<Self>) -> Result<CompiledExecutionGraph, String> {
        for indexed in &self.sections {
            if indexed.section.input_ref.is_none() {
                return Err(format!("Section {} has no input reference", indexed.index));
            }
            if indexed.section.output_ref.is_none() {
                return Err(format!("Section {} has no output reference", indexed.index));
            }
        }

        // Construction already validates producers and builds the execution layers.
        Ok(CompiledExecutionGraph {
            graph: Arc::clone(self),
            input_refs: self
                .ref_table
                .inputs
                .iter()
                .map(|r| r.name.clone())
                .collect(),
            output_refs: self
                .ref_table
                .outputs
                .iter()
                .map(|r| r.name.clone())
                .collect(),
        })
    }
}

#[pyclass]
pub struct CompiledExecutionGraph {
    pub graph: Arc<ExecutionGraph>,
    #[pyo3(get)]
    pub input_refs: Vec<String>,
    #[pyo3(get)]
    pub output_refs: Vec<String>,
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
                let mut secondary_plans = Vec::new();
                for secondary_input_name in
                    section.secondary_input_refs.as_deref().unwrap_or_default()
                {
                    // get plan from secondary_input_name
                    let ref_id = reference_id(secondary_input_name)?;
                    let plan = plans[ref_id]
                        .as_ref()
                        .ok_or_else(|| {
                            format!(
                                "Section {section_index}: secondary input '{secondary_input_name}' is not available"
                            )
                        })?
                        .clone();

                    secondary_plans.push(NamedFrame {
                        data: plan,
                        name: secondary_input_name.to_string(),
                    });
                }
                // todo load plan from secondary_plans from the refs: THAT ALSO MEANS THE CURRENT TOPOLOGICAL ALGO IS WRONGAS IT DIDNT TOOK INTO ACCOUNT SECONDARY INPUTS FOR dependencies
                for (op_index, op) in section.instructions.iter().enumerate() {
                    plan = op
                        .execute_lazy(plan, Some(secondary_plans.clone()))
                        .map_err(|err| {
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
}

#[pymethods]
impl CompiledExecutionGraph {
    /// Execute named inputs and return Polars DataFrames in output_refs order.
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
        // Workers may need the GIL to release Python-owned input buffers.
        let resolved = py
            .detach(move || {
                LazyFrame::collect_all_with_engine(
                    output_plan
                        .into_iter()
                        .map(|fr| fr.lazyframe.logical_plan)
                        .collect(),
                    polars::prelude::Engine::Auto,
                    polars::prelude::OptFlags::default(),
                )
            })
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
#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct NamedFrame {
    pub data: LazyFrame,
    #[pyo3(get)]
    pub name: String,
}

impl NamedFrame {
    pub fn new_from_lf(data: LazyFrame, name: String) -> Self {
        Self { data, name }
    }
}

#[pymethods]
impl NamedFrame {
    #[new]
    pub fn new_from_py(data: &Bound<'_, PyAny>, name: String) -> PyResult<Self> {
        let df = from_python(data)?;
        Ok(Self::new_from_lf(df.lazy(), name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::{PyJoinOp, PyOp};
    use polars::prelude::*;

    fn section(input: &str, output: &str, secondary: Option<&str>) -> PySection {
        let instructions = secondary
            .map(|name| {
                vec![PyOp::Join(PyJoinOp::new(
                    vec!["id".into()],
                    vec!["id".into()],
                    name.into(),
                    "inner".into(),
                ))]
            })
            .unwrap_or_default();
        let mut section = PySection::new(instructions);
        section.input_ref = Some(input.into());
        section.output_ref = Some(output.into());
        section.secondary_input_refs = secondary.map(|name| vec![name.into()]);
        section
    }

    fn input(graph: &CompiledExecutionGraph, name: &str, data: DataFrame) -> FramedReference {
        FramedReference {
            reference: graph.graph.ref_table.get_ref(name).unwrap().clone(),
            lazyframe: data.lazy(),
        }
    }

    fn assert_join_result(outputs: Vec<FramedReference>) {
        assert_eq!(outputs.len(), 1);
        let output = outputs.into_iter().next().unwrap();
        assert_eq!(output.reference.name, "joined");
        let result = output.lazyframe.collect().unwrap();
        let expected = df!("id" => [2i64], "amount" => [20i64], "value" => [200i64]).unwrap();
        assert!(
            result.equals(&expected),
            "unexpected join result: {result:?}"
        );
    }

    #[test]
    fn join_waits_for_secondary_input_produced_by_later_section() {
        let graph = Arc::new(
            ExecutionGraph::new(vec![
                section("orders", "joined", Some("customers")),
                section("raw_customers", "customers", None),
            ])
            .unwrap(),
        );
        assert_eq!(graph.execution_layers, vec![vec![1], vec![0]]);
        assert_eq!(graph.dependency_tree, vec![vec![], vec![0]]);
        let customers = graph.ref_table.get_ref("customers").unwrap();
        assert_eq!(customers.reference_type, reference::ReferenceType::Internal);
        assert_eq!(customers.needs, vec![1]);
        assert_eq!(customers.needed, vec![0]);

        let compiled = graph.compile().unwrap();
        assert_eq!(compiled.input_refs, vec!["orders", "raw_customers"]);
        assert_eq!(compiled.output_refs, vec!["joined"]);
        let outputs = compiled
            .apply_plan(vec![
                input(
                    &compiled,
                    "orders",
                    df!("id" => [1i64, 2], "amount" => [10i64, 20]).unwrap(),
                ),
                input(
                    &compiled,
                    "raw_customers",
                    df!("id" => [2i64, 3], "value" => [200i64, 300]).unwrap(),
                ),
            ])
            .unwrap();
        assert_join_result(outputs);
    }

    #[test]
    fn join_accepts_external_secondary_input() {
        let graph = Arc::new(
            ExecutionGraph::new(vec![section("orders", "joined", Some("customers"))]).unwrap(),
        );
        assert_eq!(graph.execution_layers, vec![vec![0]]);
        let compiled = graph.compile().unwrap();
        assert_eq!(compiled.input_refs, vec!["orders", "customers"]);
        let outputs = compiled
            .apply_plan(vec![
                input(
                    &compiled,
                    "customers",
                    df!("id" => [2i64, 3], "value" => [200i64, 300]).unwrap(),
                ),
                input(
                    &compiled,
                    "orders",
                    df!("id" => [1i64, 2], "amount" => [10i64, 20]).unwrap(),
                ),
            ])
            .unwrap();
        assert_join_result(outputs);
    }

    #[test]
    fn join_rejects_missing_external_secondary_input() {
        let graph = Arc::new(
            ExecutionGraph::new(vec![section("orders", "joined", Some("customers"))]).unwrap(),
        );
        let compiled = graph.compile().unwrap();
        let error = compiled
            .apply_plan(vec![input(
                &compiled,
                "orders",
                df!("id" => [1i64]).unwrap(),
            )])
            .err()
            .expect("missing secondary input must be rejected");
        assert_eq!(error, "Missing input 'customers'");
    }

    #[test]
    fn unavailable_secondary_plan_returns_contextual_error() {
        let mut graph = ExecutionGraph::new(vec![
            section("orders", "joined", Some("customers")),
            section("raw_customers", "customers", None),
        ])
        .unwrap();
        // Simulate a malformed schedule that runs the consumer before its producer.
        graph.execution_layers = vec![vec![0], vec![1]];
        let compiled = Arc::new(graph).compile().unwrap();
        let error = compiled
            .apply_plan(vec![
                input(&compiled, "orders", df!("id" => [1i64]).unwrap()),
                input(&compiled, "raw_customers", df!("id" => [1i64]).unwrap()),
            ])
            .err()
            .expect("unavailable secondary plan must return an error, not panic");
        assert_eq!(
            error,
            "Section 0: secondary input 'customers' is not available"
        );
    }
}
