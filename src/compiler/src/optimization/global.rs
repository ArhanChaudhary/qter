use std::collections::HashMap;

use internment::ArcIntern;
use itertools::Itertools;
use qter_core::WithSpan;

use crate::{LabelReference, optimization::OptimizingPrimitive, primitive_match};

use super::OptimizingCodeComponent;

/// Returns a bool to indicate whether the output is the same as the input
pub fn do_global_optimization(
    instructions: impl Iterator<Item = WithSpan<OptimizingCodeComponent>> + Send + 'static,
) -> (Vec<WithSpan<OptimizingCodeComponent>>, bool) {
    let mut label_locations = HashMap::new();
    let mut program_counter = 0;

    let instructions = instructions
        .inspect(|component| {
            if let OptimizingCodeComponent::Label(label) = &**component {
                label_locations.insert(
                    LabelReference {
                        name: ArcIntern::clone(&label.name),
                        block_id: label.maybe_block_id.unwrap(),
                    },
                    false,
                );
            }

            program_counter += 1;
        })
        .collect_vec();

    for instruction in &instructions {
        primitive_match!((OptimizingPrimitive::Goto { label } | OptimizingPrimitive::SolvedGoto { label, .. }) = &**instruction; else { continue; });

        let Some(is_seen) = label_locations.get_mut(&LabelReference {
            name: ArcIntern::clone(&label.name),
            block_id: label.block_id,
        }) else {
            continue;
        };

        *is_seen = true;
    }

    let mut convergence = true;

    (
        instructions
            .into_iter()
            .filter(|component| {
                let OptimizingCodeComponent::Label(label) = &**component else {
                    return true;
                };

                let jumped_to = *label_locations
                    .get(&LabelReference {
                        name: ArcIntern::clone(&label.name),
                        block_id: label.maybe_block_id.unwrap(),
                    })
                    .unwrap();

                convergence &= jumped_to;

                jumped_to
            })
            .collect_vec(),
        convergence,
    )
}
