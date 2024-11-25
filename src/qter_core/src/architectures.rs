use std::{cell::OnceCell, collections::HashMap, rc::Rc};

use bnum::{cast::As, types::U512};
use internment::ArcIntern;
use itertools::Itertools;

use crate::{
    discrete_math::chinese_remainder_theorem, puzzle_parser,
    shared_facelet_detection::algorithms_to_cycle_generators,
};

#[derive(Debug)]
pub struct PuzzleDefinition {
    pub group: Rc<PermutationGroup>,
    pub presets: Vec<Rc<Architecture>>,
}

impl PuzzleDefinition {
    pub fn parse(spec: &str) -> Result<PuzzleDefinition, String> {
        puzzle_parser::parse(spec).map_err(|v| v.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct PermutationGroup {
    facelet_colors: Vec<ArcIntern<String>>,
    generators: HashMap<ArcIntern<String>, Permutation>,
}

impl PermutationGroup {
    pub fn new(
        facelet_colors: Vec<ArcIntern<String>>,
        mut generators: HashMap<ArcIntern<String>, Permutation>,
    ) -> PermutationGroup {
        assert!(!generators.is_empty());

        for generator in generators.values() {
            assert!(generator.facelet_count <= facelet_colors.len());
        }

        for generator in generators.iter_mut() {
            generator.1.facelet_count = facelet_colors.len();
        }

        PermutationGroup {
            facelet_colors,
            generators,
        }
    }

    pub fn facelet_count(&self) -> usize {
        self.facelet_colors.len()
    }

    pub fn facelet_colors(&self) -> &[ArcIntern<String>] {
        &self.facelet_colors
    }

    pub fn identity(&self) -> Permutation {
        Permutation {
            // Map every value to itself
            mapping: OnceCell::from((0..self.facelet_count()).collect::<Vec<_>>()),
            cycles: OnceCell::new(),
            facelet_count: self.facelet_count(),
        }
    }

    pub fn generators(&self) -> impl Iterator<Item = (&str, &Permutation)> {
        self.generators.iter().map(|(k, v)| (&***k, v))
    }

    pub fn get_generator(&self, name: &str) -> Option<&Permutation> {
        self.generators.get(&ArcIntern::from_ref(name))
    }

    /// If any of the generator names don't exist, it will compose all of the generators before it and return the name of the generator that doesn't exist.
    pub fn compose_generators_into<'a, S: AsRef<str>>(
        &self,
        permutation: &mut Permutation,
        generators: &'a [S],
    ) -> Result<(), &'a S> {
        for generator in generators {
            let generator = match self
                .generators
                .get(&ArcIntern::from_ref(generator.as_ref()))
            {
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
    pub(crate) facelet_count: usize,
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

    pub fn exponentiate(&mut self, power: U512) {
        self.cycles();
        let mut mapping = self
            .mapping
            .take()
            .unwrap_or_else(|| (0..self.facelet_count).collect_vec());
        let cycles = self.cycles();

        for cycle in cycles {
            let len = U512::from_digit(cycle.len() as u64);
            for i in 0..cycle.len() {
                mapping[i] = cycle[(U512::from_digit(i as u64) + power).rem(len).as_::<usize>()];
            }
        }

        self.mapping = OnceCell::from(mapping);
        self.cycles = OnceCell::new();
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

#[derive(PartialEq, Eq, Debug)]
pub struct CycleGeneratorSubcycle {
    pub(crate) facelet_cycle: Vec<usize>,
    pub(crate) chromatic_order: U512,
}

impl CycleGeneratorSubcycle {
    pub fn facelet_cycle(&self) -> &[usize] {
        &self.facelet_cycle
    }

    pub fn chromatic_order(&self) -> U512 {
        self.chromatic_order
    }
}

#[derive(Debug)]
pub struct CycleGenerator {
    pub(crate) generator_sequence: Vec<String>,
    pub(crate) permutation: Permutation,
    pub(crate) unshared_cycles: Vec<CycleGeneratorSubcycle>,
    pub(crate) order: U512,
    pub(crate) group: Rc<PermutationGroup>,
}

impl CycleGenerator {
    pub fn generator_sequence(&self) -> &[String] {
        &self.generator_sequence
    }

    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    pub fn unshared_cycles(&self) -> &[CycleGeneratorSubcycle] {
        &self.unshared_cycles
    }

    pub fn order(&self) -> U512 {
        self.order
    }

    pub fn is_solved(&self, permutation: &Permutation) -> bool {
        let mapping = permutation.mapping();

        self.unshared_cycles()
            .iter()
            .flat_map(|v| v.facelet_cycle())
            .all(|v| self.group.facelet_colors()[mapping[*v]] == self.group.facelet_colors[*v])
    }

    pub fn decode(&self, permutation: &Permutation) -> U512 {
        chinese_remainder_theorem(
            self.unshared_cycles()
                .iter()
                .map(|v| {
                    let cycle = v.facelet_cycle();

                    let offset = U512::from_digit(
                        cycle
                            .iter()
                            .find_position(|v| **v == permutation.mapping()[cycle[0]])
                            .unwrap()
                            .0 as u64,
                    )
                    .rem(v.chromatic_order());

                    (offset, v.chromatic_order())
                })
                .collect_vec(),
        )
    }
}

#[derive(Debug)]
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
        let processed = algorithms_to_cycle_generators(Rc::clone(&group), &algorithms)?;

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
