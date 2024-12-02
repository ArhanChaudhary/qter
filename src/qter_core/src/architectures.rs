use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use bnum::{cast::As, types::U512};
use internment::ArcIntern;
use itertools::Itertools;

use crate::{
    discrete_math::lcm_iter, puzzle_parser,
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

    // If they want the cycles in a different order, create a new architecture with the cycles shuffled
    fn adapt_architecture(
        architecture: &Rc<Architecture>,
        orders: &[U512],
    ) -> Option<Rc<Architecture>> {
        let mut used = vec![false; orders.len()];
        let mut swizzle = vec![0; orders.len()];

        for (i, order) in orders.iter().enumerate() {
            let mut found_one = false;

            for (j, cycle) in architecture.cycle_generators.iter().enumerate() {
                if !used[j] && cycle.order() == *order {
                    used[j] = true;
                    found_one = true;
                    swizzle[i] = j;
                }
            }

            if !found_one {
                return None;
            }
        }

        if swizzle.iter().enumerate().all(|(v, i)| v == *i) {
            return Some(Rc::clone(architecture));
        }

        let mut new_arch = Architecture::clone(architecture);

        for i in 0..swizzle.len() {
            new_arch.cycle_generators.swap(i, swizzle[i]);

            for j in i..swizzle.len() {
                if i == swizzle[j] {
                    swizzle[j] = swizzle[i];
                    break;
                }
            }
        }

        Some(Rc::new(new_arch))
    }

    pub fn get_preset(&self, orders: &[U512]) -> Option<Rc<Architecture>> {
        for preset in &self.presets {
            if preset.cycle_generators.len() != orders.len() {
                continue;
            }

            if let Some(arch) = Self::adapt_architecture(preset, orders) {
                return Some(arch);
            }
        }

        None
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
    pub fn compose_generators_into<'a>(
        &self,
        permutation: &mut Permutation,
        generators: impl Iterator<Item = &'a ArcIntern<String>>,
    ) -> Result<(), ArcIntern<String>> {
        for generator in generators {
            let generator = match self
                .generators
                .get(&ArcIntern::from_ref(generator.as_ref()))
            {
                Some(idx) => idx,
                None => return Err(ArcIntern::clone(generator)),
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
                mapping[cycle[i]] =
                    cycle[(U512::from_digit(i as u64) + power).rem(len).as_::<usize>()];
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

impl PartialEq for Permutation {
    fn eq(&self, other: &Self) -> bool {
        self.mapping() == other.mapping()
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct CycleGenerator {
    pub(crate) generator_sequence: Vec<ArcIntern<String>>,
    pub(crate) permutation: Permutation,
    pub(crate) unshared_cycles: Vec<CycleGeneratorSubcycle>,
    pub(crate) order: U512,
    pub(crate) group: Rc<PermutationGroup>,
}

impl CycleGenerator {
    pub fn generator_sequence(&self) -> &[ArcIntern<String>] {
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

    pub fn signature_facelets(&self) -> Vec<usize> {
        let mut cycles_with_extras = vec![];

        for (i, cycle) in self.unshared_cycles().iter().enumerate() {
            if cycle.chromatic_order() != U512::ONE {
                cycles_with_extras.push((cycle.chromatic_order(), i));
            }
        }

        cycles_with_extras.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let mut cycles = Vec::<(U512, usize)>::new();

        for (i, (cycle_order, cycle_idx)) in cycles_with_extras.iter().enumerate() {
            if self.order()
                != lcm_iter(
                    cycles.iter().map(|v| v.0).chain(
                        (i + 1..cycles_with_extras.len()).map(|idx| cycles_with_extras[idx].0),
                    ),
                )
            {
                cycles.push((*cycle_order, *cycle_idx));
            }
        }

        let mut facelets = vec![];

        for (_, idx) in cycles {
            let cycle = &self.unshared_cycles()[idx];
            let chromatic_order = cycle.chromatic_order().digits()[0] as usize;

            let mut uncovered = HashSet::<usize>::from_iter(1..chromatic_order);

            let mut facelet_idx = 0;
            while !uncovered.is_empty() {
                let facelet = cycle.facelet_cycle()[facelet_idx];
                let mut still_uncovered = HashSet::new();

                for i in 1..chromatic_order {
                    if self.group.facelet_colors()
                        [cycle.facelet_cycle()[(i + facelet_idx) % chromatic_order]]
                        == self.group.facelet_colors()[facelet]
                    {
                        still_uncovered.insert(i);
                    }
                }

                if !uncovered.is_subset(&still_uncovered) {
                    uncovered.retain(|v| still_uncovered.contains(v));
                    facelets.push(facelet);
                }

                facelet_idx += 1;
            }
        }

        facelets
    }
}

#[derive(Debug, Clone)]
pub struct Architecture {
    group: Rc<PermutationGroup>,
    cycle_generators: Vec<CycleGenerator>,
    shared_facelets: Vec<usize>,
}

impl Architecture {
    pub fn new(
        group: Rc<PermutationGroup>,
        algorithms: Vec<Vec<ArcIntern<String>>>,
    ) -> Result<Architecture, ArcIntern<String>> {
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

    pub fn group_rc(&self) -> Rc<PermutationGroup> {
        Rc::clone(&self.group)
    }

    pub fn registers(&self) -> &[CycleGenerator] {
        &self.cycle_generators
    }

    pub fn shared_facelets(&self) -> &[usize] {
        &self.shared_facelets
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use bnum::types::U512;
    use internment::ArcIntern;
    use itertools::Itertools;

    use super::{Architecture, PuzzleDefinition};

    #[test]
    fn three_by_three() {
        let cube = PuzzleDefinition::parse(include_str!("../puzzles/3x3.txt")).unwrap();

        for (arch, expected) in [
            (&["U", "D"][..], &[4, 4][..]),
            (
                &["R' F' L U' L U L F U' R", "U F R' D' R2 F R' U' D"],
                &[90, 90],
            ),
            (
                &["U R U' D2 B", "B U2 B' L' U2 B U L' B L B2 L"],
                &[210, 24],
            ),
            (
                &[
                    "U L2 B' L U' B' U2 R B' R' B L",
                    "R2 L U' R' L2 F' D R' D L B2 D2",
                    "L2 F2 U L' F D' F' U' L' F U D L' U'",
                ],
                &[30, 30, 30],
            ),
        ]
        .iter()
        {
            let arch = Architecture::new(
                Rc::clone(&cube.group),
                arch.iter()
                    .map(|v| v.split(" ").map(ArcIntern::from_ref).collect_vec())
                    .collect_vec(),
            )
            .unwrap();

            for (register, expected) in arch.cycle_generators.iter().zip(expected.iter()) {
                assert_eq!(register.order(), U512::from_digit(*expected as u64));
            }
        }
    }

    #[test]
    fn exponentiation() {
        let cube = PuzzleDefinition::parse(include_str!("../puzzles/3x3.txt")).unwrap();

        let mut perm = cube.group.identity();

        cube.group
            .compose_generators_into(
                &mut perm,
                [ArcIntern::from_ref("U"), ArcIntern::from_ref("L")].iter(),
            )
            .unwrap();

        let mut exp_perm = perm.clone();
        exp_perm.exponentiate(U512::from_digit(7));

        let mut repeat_compose_perm = cube.group.identity();

        repeat_compose_perm.compose(&perm);
        repeat_compose_perm.compose(&perm);
        repeat_compose_perm.compose(&perm);
        repeat_compose_perm.compose(&perm);
        repeat_compose_perm.compose(&perm);
        repeat_compose_perm.compose(&perm);
        repeat_compose_perm.compose(&perm);

        assert_eq!(exp_perm, repeat_compose_perm);
    }
}
