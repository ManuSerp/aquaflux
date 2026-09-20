use crate::graph::reference::ReferenceTable;
use crate::interface::section::PySection;
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
    pub section: PySection,
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
            .map(|(i, section)| IndexedSection { index: i, section })
            .collect();
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
