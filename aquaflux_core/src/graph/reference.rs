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

    pub fn build_table(&mut self, sections: &[IndexedSection]) -> Result<(), String> {
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
            table.build_table(&sections).unwrap();
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
            .build_table(&[section(0, "source", "a"), section(1, "source", "b")])
            .unwrap();
        assert_eq!(table.inputs.len(), 1);
        assert_eq!(table.inputs[0].needed, vec![0, 1]);
        table.build_table(&[]).unwrap();
        assert!(table.references.is_empty());
        assert!(table.ref_map.is_empty());
        assert!(table.inputs.is_empty());
        assert!(table.outputs.is_empty());
    }

    #[test]
    fn rejects_duplicate_output_and_internal_producers_atomically() {
        let mut table = ReferenceTable::new();
        table
            .build_table(&[section(9, "original", "result")])
            .unwrap();
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
            let error = table.build_table(&sections).unwrap_err();
            assert!(error.contains("shared"));
            assert!(error.contains("duplicate producer"));
            assert_eq!(table.inputs[0].name, "original");
            assert!(table.get_ref("shared").is_none());
        }
    }

    #[test]
    fn retains_self_dependency_for_graph_cycle_validation() {
        let mut table = ReferenceTable::new();
        table.build_table(&[section(0, "same", "same")]).unwrap();
        let reference = table.get_ref("same").unwrap();
        assert_eq!(reference.reference_type, ReferenceType::Internal);
        assert_eq!(reference.needs, vec![0]);
        assert_eq!(reference.needed, vec![0]);
        assert_eq!(reference.section_index, vec![0]);
    }
}
