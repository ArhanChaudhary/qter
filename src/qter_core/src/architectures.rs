use std::{cell::OnceCell, collections::HashMap, rc::Rc};

use bnum::types::U512;
use itertools::Itertools;

use crate::shared_facelet_detection::algorithms_to_cycle_generators;

#[derive(Clone, Debug)]
pub struct PermutationGroup {
    facelet_count: usize,
    generators: HashMap<String, Permutation>,
}

impl PermutationGroup {
    pub fn new(mut generators: HashMap<String, Permutation>) -> PermutationGroup {
        assert!(!generators.is_empty());

        let facelet_count = generators.iter().map(|v| v.1.facelet_count).max().unwrap() + 1;

        for generator in generators.iter_mut() {
            generator.1.facelet_count = facelet_count;
        }

        PermutationGroup {
            facelet_count,
            generators,
        }
    }

    pub fn identity(&self) -> Permutation {
        Permutation {
            // Map every value to itself
            mapping: OnceCell::from((0..self.facelet_count).collect::<Vec<_>>()),
            cycles: OnceCell::new(),
            facelet_count: self.facelet_count,
        }
    }

    pub fn generators(&self) -> impl Iterator<Item = (&str, &Permutation)> {
        self.generators.iter().map(|(k, v)| (&**k, v))
    }

    pub fn facelet_count(&self) -> usize {
        self.facelet_count
    }

    pub fn get_generator(&self, name: &str) -> Option<&Permutation> {
        self.generators.get(name)
    }

    /// If any of the generator names don't exist, it will compose all of the generators before it and return the name of the generator that doesn't exist.
    pub fn compose_generators_into<'a, S: AsRef<str>>(
        &self,
        permutation: &mut Permutation,
        generators: &'a [S],
    ) -> Result<(), &'a S> {
        for generator in generators {
            let generator = match self.generators.get(generator.as_ref()) {
                Some(idx) => idx,
                None => return Err(generator),
            };

            permutation.compose(generator);
        }

        Ok(())
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
    pub fn from_cycles(mut cycles: Vec<Vec<usize>>) -> Permutation {
        cycles.retain(|v| v.len() > 1);

        assert!(!cycles.is_empty());

        let facelet_count = *cycles.iter().flatten().max().unwrap() + 1;

        Permutation {
            facelet_count,
            mapping: OnceCell::new(),
            cycles: OnceCell::from(cycles),
        }
    }

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

                if cycle.len() > 1 {
                    cycles.push(cycle);
                }
            }

            cycles
        })
    }

    fn mapping_mut(&mut self) -> &mut [usize] {
        self.mapping();

        return self.mapping.get_mut().unwrap();
    }

    pub fn compose(&mut self, other: &Permutation) {
        assert_eq!(self.facelet_count, other.facelet_count);

        let my_mapping = self.mapping_mut();
        let other_mapping = other.mapping();

        for value in my_mapping.iter_mut() {
            *value = other_mapping[*value];
        }

        // Invalidate `cycles`
        self.cycles = OnceCell::new();
    }
}

pub struct CycleGenerator {
    pub(crate) generator_sequence: Vec<String>,
    pub(crate) permutation: Permutation,
    pub(crate) unshared_cycles: Vec<Vec<usize>>,
    pub(crate) order: U512,
}

impl CycleGenerator {
    pub fn generator_sequence(&self) -> &[String] {
        &self.generator_sequence
    }

    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    pub fn unshared_cycles(&self) -> &[Vec<usize>] {
        &self.unshared_cycles
    }

    pub fn order(&self) -> U512 {
        self.order
    }
}

pub struct Architecture {
    group: Rc<PermutationGroup>,
    cycle_generators: Vec<CycleGenerator>,
    shared_facelets: Vec<usize>,
}

impl Architecture {
    pub fn new(
        group: Rc<PermutationGroup>,
        algorithms: Vec<Vec<String>>,
    ) -> Result<Architecture, String> {
        let processed = algorithms_to_cycle_generators(&group, &algorithms)?;

        Ok(Architecture {
            group,
            cycle_generators: processed.0,
            shared_facelets: processed.1,
        })
    }

    pub fn group(&self) -> &PermutationGroup {
        &self.group
    }

    pub fn registers(&self) -> &[CycleGenerator] {
        &self.cycle_generators
    }

    pub fn shared_facelets(&self) -> &[usize] {
        &self.shared_facelets
    }
}

pub struct Cube {
    architecture: Rc<Architecture>,
    state: Permutation,
}

impl Cube {
    pub fn architecture(&self) -> &Architecture {
        &self.architecture
    }

    pub fn state(&self) -> &Permutation {
        &self.state
    }
}
