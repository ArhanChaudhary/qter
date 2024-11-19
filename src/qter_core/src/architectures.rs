use std::{cell::OnceCell, collections::HashMap, rc::Rc};

use bnum::types::U512;
use itertools::Itertools;

#[derive(Clone, Debug)]
pub struct PermutationGroup {
    facelet_count: usize,
    generator_names: HashMap<String, usize>,
    generators: Vec<Permutation>,
}

impl PermutationGroup {
    pub fn identity(&self) -> Permutation {
        Permutation {
            // Map every value to itself
            mapping: OnceCell::from((0..self.facelet_count).collect::<Vec<_>>()),
            cycles: OnceCell::new(),
            facelet_count: self.facelet_count,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Permutation {
    facelet_count: usize,
    // One of these two must be defined
    mapping: OnceCell<Vec<usize>>,
    cycles: OnceCell<Vec<Vec<usize>>>,
}

impl Permutation {
    pub fn mapping(&self) -> &[usize] {
        self.mapping.get_or_init(|| {
            let cycles = self
                .cycles
                .get()
                .expect("either `mapping` or `cycles` to be defined");

            // Start with the identity permutation
            let mut mapping = (0..self.facelet_count).collect::<Vec<_>>();

            for cycle in cycles {
                for (start, end) in cycle.iter().cycle().tuple_windows().take(cycle.len()) {
                    mapping[*start] = *end;
                }
            }

            mapping
        })
    }

    pub fn cycles(&self) -> &[Vec<usize>] {
        self.cycles.get_or_init(|| {
            let mapping = self
                .mapping
                .get()
                .expect("either `mapping` or `cycles` to be defined");

            let mut covered = vec![false; self.facelet_count];
            let mut cycles = vec![];

            for i in 0..self.facelet_count {
                if covered[i] {
                    continue;
                }

                covered[i] = true;
                let mut cycle = vec![i];

                loop {
                    let next = mapping[*cycle.last().unwrap()];

                    if cycle[0] == next {
                        break;
                    }

                    covered[next] = true;
                    cycle.push(next);
                }

                cycles.push(cycle);
            }

            cycles
        })
    }

    pub fn compose(&self, other: &Permutation) -> Permutation {
        assert_eq!(self.facelet_count, other.facelet_count);

        let my_mapping = self.mapping();
        let other_mapping = other.mapping();

        let new_mapping = my_mapping
            .iter()
            .map(|my_maps_to| other_mapping[*my_maps_to])
            .collect::<Vec<_>>();

        Permutation {
            facelet_count: self.facelet_count,
            mapping: OnceCell::from(new_mapping),
            cycles: OnceCell::new(),
        }
    }
}

struct CycleGenerator {
    generator_sequence: Vec<usize>,
    permutation: Permutation,
    unshared_facelets: Vec<usize>,
    order: U512,
}

pub struct Architecture {
    group: Rc<PermutationGroup>,
    cycle_generators: Vec<CycleGenerator>,
    shared_facelets: Vec<usize>,
}

pub struct Cube {
    architecture: Rc<Architecture>,
    state: Permutation,
}
