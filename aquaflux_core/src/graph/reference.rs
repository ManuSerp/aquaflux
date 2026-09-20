use crate::graph::IndexedSection;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ReferenceTable {
    pub references: Vec<Reference>,
    // Indices avoid self-referential borrows and survive Vec reallocations.
    // but maybe would have been better to not haev the vec and just store in the hashmap?
    pub ref_map: HashMap<String, usize>,
    pub inputs: Vec<Reference>,
    pub outputs: Vec<Reference>,
}

#[derive(Clone, Debug)]
pub struct Reference {
    pub name: String,
    pub reference_type: ReferenceType,
    pub section_index: Vec<usize>,
    pub needs: Vec<usize>,  // producing section (at most one)
    pub needed: Vec<usize>, // consuming sections
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceType {
    Input,
    Output,
    Internal,
}

impl ReferenceTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_ref(&self, name: &str) -> Option<&Reference> {
        self.ref_map.get(name).map(|&index| &self.references[index])
    }

    fn get_ref_mut(&mut self, name: &str) -> Option<&mut Reference> {
        let index = *self.ref_map.get(name)?;
        Some(&mut self.references[index])
    }

    fn add_ref(&mut self, reference: Reference) {
        self.ref_map
            .insert(reference.name.clone(), self.references.len());
        self.references.push(reference);
    }

    pub fn build(&mut self, sections: &[IndexedSection]) -> Result<(), String> {
        // Rebuild independently so a failure leaves the previous table intact.
        let mut table = Self::new();
        for isection in sections {
            if let Some(input) = &isection.section.input_ref {
                if let Some(reference) = table.get_ref_mut(input) {
                    if reference.reference_type == ReferenceType::Output {
                        reference.reference_type = ReferenceType::Internal;
                    }
                    if !reference.section_index.contains(&isection.index) {
                        reference.section_index.push(isection.index);
                    }
                    if !reference.needed.contains(&isection.index) {
                        reference.needed.push(isection.index);
                    }
                } else {
                    table.add_ref(Reference {
                        name: input.clone(),
                        reference_type: ReferenceType::Input,
                        section_index: vec![isection.index],
                        needs: Vec::new(),
                        needed: vec![isection.index],
                    });
                }
            }

            if let Some(output) = &isection.section.output_ref {
                if let Some(reference) = table.get_ref_mut(output) {
                    // Internal references already have a producer too.
                    if !reference.needs.is_empty() {
                        return Err(format!(
                            "Output reference '{}' already produced by section {}; duplicate producer section {} ({})",
                            output,
                            reference.needs[0],
                            isection.index,
                            isection.section.name.as_deref().unwrap_or("unnamed"),
                        ));
                    }
                    reference.reference_type = ReferenceType::Internal;
                    if !reference.section_index.contains(&isection.index) {
                        reference.section_index.push(isection.index);
                    }
                    reference.needs.push(isection.index);
                } else {
                    table.add_ref(Reference {
                        name: output.clone(),
                        reference_type: ReferenceType::Output,
                        section_index: vec![isection.index],
                        needs: vec![isection.index],
                        needed: Vec::new(),
                    });
                }
            }
        }

        for reference in &table.references {
            match reference.reference_type {
                ReferenceType::Input => table.inputs.push(reference.clone()),
                ReferenceType::Output => table.outputs.push(reference.clone()),
                ReferenceType::Internal => {}
            }
        }
        *self = table;
        Ok(())
    }

    /// Builds deterministic execution layers using Kahn's topological-sort algorithm.
    ///
    /// # Arguments and prerequisites
    ///
    /// `section_count` is the total number of sections, with stable indices in
    /// `0..section_count`. Include sections without references: they are invisible
    /// to the reference table but must still be scheduled. The table should have
    /// been populated by [`Self::build`] from the same set of sections.
    ///
    /// # Return value
    ///
    /// Returns `(layers, children)`, both containing original section indices:
    ///
    /// - `layers[n]` contains sections eligible to run in parallel after every
    ///   preceding layer has finished. Indices within each layer are sorted.
    /// - `children[producer]` contains its sorted, unique direct consumers. This
    ///   adjacency list represents a DAG, not a tree: a section can have several
    ///   parents and several children. Isolated sections have an empty list.
    ///
    /// For example, edges `0 -> 2`, `1 -> 2`, and `2 -> 3` yield:
    ///
    /// ```text
    /// layers   = [[0, 1], [2], [3]]
    /// children = [[2], [2], [3], []]
    /// ```
    ///
    /// An empty graph returns two empty vectors. If there are sections but no
    /// internal dependencies, all sections belong to layer zero.
    ///
    /// # Algorithm
    ///
    /// 1. Convert each reference's producer (`needs`) and consumers (`needed`)
    ///    into directed producer-to-consumer edges. An external input has no
    ///    section producer, so it introduces no scheduling dependency.
    /// 2. Deduplicate edges before counting each section's incoming edges
    ///    (its in-degree). Consuming several datasets from the same producer
    ///    still means waiting for only one prerequisite section.
    /// 3. Start with all zero-in-degree sections as the first ready layer.
    /// 4. Remove that entire layer logically: decrement the in-degree of each
    ///    child. Children reaching zero form the next layer, never the current
    ///    one. Repeat until no sections are ready.
    /// 5. If fewer than `section_count` sections were scheduled, reject the graph
    ///    because at least one cycle prevents a complete topological ordering.
    ///
    /// This assigns every section its earliest possible layer: zero without
    /// internal prerequisites, otherwise one plus the maximum parent layer.
    /// Parents may belong to any earlier layer, not just the immediately
    /// preceding one. A terminal output on a short branch can therefore be
    /// produced before the final layer; terminal sections are not delayed.
    /// Declaration order does not affect the result for fixed section indices.
    ///
    /// # Errors
    ///
    /// Returns an error if a reference contains an out-of-range section index,
    /// has multiple producers, or the graph contains a cycle (including a
    /// section consuming its own output). Cycle errors report blocked section
    /// indices and associated reference names. Blocked sections can include
    /// downstream dependents, not only sections participating in a cycle; this
    /// algorithm does not reconstruct an exact cycle path.
    ///
    /// # Complexity and scope
    ///
    /// Kahn's traversal itself is `O(V + E)` for `V` sections and `E` unique
    /// edges. Sorting adjacency lists for deduplication and sorting ready layers
    /// adds sorting costs. Storage is `O(V + R)`, where `R` is the number of
    /// producer-to-consumer edges before deduplication.
    ///
    /// This method does not mutate the table or execute sections. It describes
    /// dependency-level parallelism; an executor must separately choose how to
    /// run a layer and ensure external datasets are available. Only dependencies
    /// recorded in this table participate, so auxiliary operation inputs must
    /// also be registered before this method can account for them.
    pub fn build_execution_layers(
        &self,
        section_count: usize,
    ) -> Result<(Vec<Vec<usize>>, Vec<Vec<usize>>), String> {
        let mut children = vec![Vec::new(); section_count];
        for reference in &self.references {
            for &index in reference
                .section_index
                .iter()
                .chain(&reference.needs)
                .chain(&reference.needed)
            {
                if index >= section_count {
                    return Err(format!(
                        "Reference '{}' contains invalid section index {} (section count {})",
                        reference.name, index, section_count,
                    ));
                }
            }
            if reference.needs.len() > 1 {
                return Err(format!(
                    "Reference '{}' has multiple producers",
                    reference.name
                ));
            }
            if let Some(&producer) = reference.needs.first() {
                children[producer].extend(&reference.needed);
            }
        }

        let mut remaining_dependencies = vec![0; section_count];
        for consumers in &mut children {
            consumers.sort_unstable();
            consumers.dedup();
            for &consumer in consumers.iter() {
                remaining_dependencies[consumer] += 1;
            }
        }
        let mut ready: Vec<usize> = (0..section_count)
            .filter(|&index| remaining_dependencies[index] == 0)
            .collect();
        let mut layers = Vec::new();
        let mut scheduled = 0;
        while !ready.is_empty() {
            let mut next = Vec::new();
            for &producer in &ready {
                for &consumer in &children[producer] {
                    remaining_dependencies[consumer] -= 1;
                    if remaining_dependencies[consumer] == 0 {
                        next.push(consumer);
                    }
                }
            }
            scheduled += ready.len();
            layers.push(ready);
            // A newly ready child belongs to the next layer, never this one.
            next.sort_unstable();
            ready = next;
        }
        if scheduled != section_count {
            let blocked: Vec<usize> = (0..section_count)
                .filter(|&index| remaining_dependencies[index] != 0)
                .collect();
            let references: Vec<&str> = self
                .references
                .iter()
                .filter(|reference| {
                    reference
                        .needs
                        .iter()
                        .any(|&index| remaining_dependencies[index] != 0)
                })
                .map(|reference| reference.name.as_str())
                .collect();
            return Err(format!(
                "Cycle detected: blocked sections {:?} (including downstream dependents), references {:?}",
                blocked, references,
            ));
        }
        Ok((layers, children))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::section::PySection;

    fn section(index: usize, input: &str, output: &str) -> IndexedSection {
        IndexedSection {
            index,
            section: PySection {
                instructions: Vec::new(),
                name: Some(format!("section_{index}")),
                input_ref: Some(input.into()),
                output_ref: Some(output.into()),
            },
        }
    }

    #[test]
    fn classifies_forward_references_and_shared_consumers() {
        for sections in [
            vec![
                section(0, "source", "shared"),
                section(1, "shared", "a"),
                section(2, "shared", "b"),
            ],
            vec![
                section(1, "shared", "a"),
                section(2, "shared", "b"),
                section(0, "source", "shared"),
            ],
        ] {
            let mut table = ReferenceTable::new();
            table.build(&sections).unwrap();
            assert_eq!(table.inputs.len(), 1);
            assert_eq!(table.inputs[0].name, "source");
            assert_eq!(table.outputs.len(), 2);
            let shared = table.get_ref("shared").unwrap();
            assert_eq!(shared.reference_type, ReferenceType::Internal);
            assert_eq!(shared.needs, vec![0]);
            assert_eq!(shared.needed, vec![1, 2]);
        }
    }

    #[test]
    fn shares_external_inputs_and_replaces_previous_build() {
        let mut table = ReferenceTable::new();
        table
            .build(&[section(0, "source", "a"), section(1, "source", "b")])
            .unwrap();
        assert_eq!(table.inputs.len(), 1);
        assert_eq!(table.inputs[0].needed, vec![0, 1]);
        table.build(&[]).unwrap();
        assert!(table.references.is_empty());
        assert!(table.ref_map.is_empty());
        assert!(table.inputs.is_empty());
        assert!(table.outputs.is_empty());
    }

    #[test]
    fn rejects_duplicate_output_and_internal_producers_atomically() {
        let mut table = ReferenceTable::new();
        table.build(&[section(9, "original", "result")]).unwrap();
        for sections in [
            vec![
                section(0, "source", "shared"),
                section(1, "other", "shared"),
            ],
            vec![
                section(0, "source", "shared"),
                section(1, "shared", "result"),
                section(2, "other", "shared"),
            ],
        ] {
            let error = table.build(&sections).unwrap_err();
            assert!(error.contains("shared"));
            assert!(error.contains("duplicate producer"));
            assert_eq!(table.inputs[0].name, "original");
            assert!(table.get_ref("shared").is_none());
        }
    }

    #[test]
    fn layers_include_empty_and_isolated_graphs() {
        let table = ReferenceTable::new();
        assert_eq!(table.build_execution_layers(0).unwrap(), (vec![], vec![]));
        assert_eq!(
            table.build_execution_layers(3).unwrap(),
            (vec![vec![0, 1, 2]], vec![vec![], vec![], vec![]])
        );
    }

    #[test]
    fn layers_are_independent_of_declaration_order() {
        let mut table = ReferenceTable::new();
        table
            .build(&[
                section(3, "b", "end"),
                section(2, "a", "b"),
                section(1, "external", "short"),
                section(0, "external", "a"),
                section(4, "a", "branch"),
            ])
            .unwrap();
        assert_eq!(
            table.build_execution_layers(6).unwrap(),
            (
                vec![vec![0, 1, 5], vec![2, 4], vec![3]],
                vec![vec![2, 4], vec![], vec![3], vec![], vec![], vec![]],
            )
        );
    }

    #[test]
    fn layers_wait_for_all_parents_and_deduplicate_edges() {
        // Synthetic references model auxiliary inputs not yet exposed by PySection.
        let mut table = ReferenceTable::new();
        for (name, producer, consumers) in [
            ("a", 0, vec![2, 1, 1]),
            ("a_again", 0, vec![2]),
            ("b", 1, vec![2]),
            ("c", 2, vec![3]),
        ] {
            table.add_ref(Reference {
                name: name.into(),
                reference_type: ReferenceType::Internal,
                section_index: vec![producer],
                needs: vec![producer],
                needed: consumers,
            });
        }
        assert_eq!(
            table.build_execution_layers(4).unwrap(),
            (
                vec![vec![0], vec![1], vec![2], vec![3]],
                vec![vec![1, 2], vec![2], vec![3], vec![]],
            )
        );
    }

    #[test]
    fn rejects_cycles_and_self_dependencies() {
        for sections in [
            vec![section(0, "same", "same")],
            vec![
                section(0, "b", "a"),
                section(1, "a", "b"),
                section(2, "b", "downstream"),
                section(3, "external", "independent"),
            ],
        ] {
            let mut table = ReferenceTable::new();
            table.build(&sections).unwrap();
            let error = table.build_execution_layers(sections.len()).unwrap_err();
            assert!(error.contains("Cycle detected"));
            assert!(error.contains("0"));
            assert!(error.contains(sections[0].section.output_ref.as_ref().unwrap()));
        }
    }

    #[test]
    fn rejects_out_of_range_producers_and_consumers() {
        for sections in [
            vec![section(2, "source", "result")],
            vec![
                section(0, "source", "shared"),
                section(2, "shared", "result"),
            ],
        ] {
            let mut table = ReferenceTable::new();
            table.build(&sections).unwrap();
            assert!(
                table
                    .build_execution_layers(2)
                    .unwrap_err()
                    .contains("invalid section index 2")
            );
        }
    }

    #[test]
    fn retains_self_dependency_for_graph_cycle_validation() {
        let mut table = ReferenceTable::new();
        table.build(&[section(0, "same", "same")]).unwrap();
        let reference = table.get_ref("same").unwrap();
        assert_eq!(reference.reference_type, ReferenceType::Internal);
        assert_eq!(reference.needs, vec![0]);
        assert_eq!(reference.needed, vec![0]);
        assert_eq!(reference.section_index, vec![0]);
    }
}
