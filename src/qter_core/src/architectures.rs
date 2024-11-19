use std::{collections::HashMap, sync::Arc};

use bnum::types::U512;
use itertools::Itertools;
use once_cell::race::OnceBox;

fn oncebox_new_with<T>(v: T) -> OnceBox<T> {
    let oncebox = OnceBox::new();
    oncebox.set(Box::new(v));
    oncebox
}

fn oncebox_clone<T: Clone>(v: &OnceBox<T>) -> OnceBox<T> {
    match v.get() {
        Some(v) => oncebox_new_with(v.clone()),
        None => OnceBox::new(),
    }
}

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
            mapping: oncebox_new_with((0..self.facelet_count).collect::<Vec<_>>()),
            cycles: OnceBox::new(),
            facelet_count: self.facelet_count,
        }
    }
}

#[derive(Debug)]
pub struct Permutation {
    facelet_count: usize,
    // One of these two must be defined
    mapping: OnceBox<Vec<usize>>,
    cycles: OnceBox<Vec<Vec<usize>>>,
}

impl Clone for Permutation {
    fn clone(&self) -> Self {
        Permutation {
            facelet_count: self.facelet_count,
            mapping: oncebox_clone(&self.mapping),
            cycles: oncebox_clone(&self.cycles),
        }
    }
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

            Box::new(mapping)
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

            Box::new(cycles)
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
            mapping: oncebox_new_with(new_mapping),
            cycles: OnceBox::new(),
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
    group: Arc<PermutationGroup>,
    cycle_generators: Vec<CycleGenerator>,
    shared_facelets: Vec<usize>,
}

pub struct Cube {
    architecture: Arc<Architecture>,
    state: Permutation,
}
