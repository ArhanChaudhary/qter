use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, OnceLock},
};

use internment::ArcIntern;
use itertools::Itertools;

use crate::{
    discrete_math::{decode, lcm_iter},
    puzzle_parser,
    shared_facelet_detection::algorithms_to_cycle_generators,
    table_encoding, Int, PermutePuzzle, I, U,
};

pub(crate) const OPTIMIZED_TABLES: [&[u8]; 0] = [];

/// The definition of a puzzle parsed from the custom format
#[derive(Debug)]
pub struct PuzzleDefinition {
    /// The permutation group of the puzzle
    pub group: Arc<PermutationGroup>,
    /// A list of preset architectures
    pub presets: Vec<Arc<Architecture>>,
}

impl PuzzleDefinition {
    /// Parse a puzzle from the spec
    pub fn parse(spec: &str) -> Result<PuzzleDefinition, String> {
        puzzle_parser::parse(spec).map_err(|v| v.to_string())
    }

    // If they want the cycles in a different order, create a new architecture with the cycles shuffled
    fn adapt_architecture(
        architecture: &Arc<Architecture>,
        orders: &[Int<U>],
    ) -> Option<Arc<Architecture>> {
        let mut used = vec![false; orders.len()];
        let mut swizzle = vec![0; orders.len()];

        for (i, order) in orders.iter().enumerate() {
            let mut found_one = false;

            for (j, cycle) in architecture.cycle_generators.iter().enumerate() {
                if !used[j] && cycle.order() == *order {
                    used[j] = true;
                    found_one = true;
                    swizzle[i] = j;
                    break;
                }
            }

            if !found_one {
                return None;
            }
        }

        if swizzle.iter().enumerate().all(|(v, i)| v == *i) {
            return Some(Arc::clone(architecture));
        }

        let mut new_arch = Architecture::clone(architecture);

        new_arch.decoded_table = OnceLock::new();

        for i in 0..swizzle.len() {
            new_arch.cycle_generators.swap(i, swizzle[i]);

            for j in i..swizzle.len() {
                if i == swizzle[j] {
                    swizzle[j] = swizzle[i];
                    break;
                }
            }
        }

        Some(Arc::new(new_arch))
    }

    /// Find a preset with the specified cycle orders
    pub fn get_preset(&self, orders: &[Int<U>]) -> Option<Arc<Architecture>> {
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

/// A permutation subgroup defined by a set of generators along with the color of each facelet
#[derive(Clone, Debug)]
pub struct PermutationGroup {
    facelet_colors: Vec<ArcIntern<str>>,
    generators: HashMap<ArcIntern<str>, Permutation>,
    generator_inverses: HashMap<ArcIntern<str>, ArcIntern<str>>,
}

impl PermutationGroup {
    /// Construct a new `PermutationGroup` from a list of facelet colors and generator permutations.
    ///
    /// This function will panic if a permutation does not include an inverse generator for each generator.
    pub fn new(
        facelet_colors: Vec<ArcIntern<str>>,
        mut generators: HashMap<ArcIntern<str>, Permutation>,
    ) -> PermutationGroup {
        assert!(!generators.is_empty());

        for generator in generators.values() {
            assert!(generator.facelet_count <= facelet_colors.len());
        }

        for generator in generators.iter_mut() {
            generator.1.facelet_count = facelet_colors.len();
        }

        let mut generator_inverses = HashMap::new();

        'next_item: for (name, generator) in generators.iter() {
            let mut inverse = generator.to_owned();
            inverse.exponentiate(Int::from(-1));
            for (name2, generator2) in generators.iter() {
                if *generator2 == inverse {
                    generator_inverses.insert(ArcIntern::clone(name), ArcIntern::clone(name2));
                    continue 'next_item;
                }
            }

            panic!("The generator {name} does not have an inverse generator");
        }

        PermutationGroup {
            facelet_colors,
            generators,
            generator_inverses,
        }
    }

    /// The number of facelets in the permutation group
    pub fn facelet_count(&self) -> usize {
        self.facelet_colors.len()
    }

    /// The colors of every facelet
    pub fn facelet_colors(&self) -> &[ArcIntern<str>] {
        &self.facelet_colors
    }

    /// The identity/solved permutation of the group
    pub fn identity(&self) -> Permutation {
        Permutation {
            // Map every value to itself
            mapping: OnceLock::from((0..self.facelet_count()).collect::<Vec<_>>()),
            cycles: OnceLock::new(),
            facelet_count: self.facelet_count(),
        }
    }

    /// Get a generator by it's name
    pub fn get_generator(&self, name: &str) -> Option<&Permutation> {
        self.generators.get(&ArcIntern::from(name))
    }

    /// Compose a list of generators into an existing permutation
    ///
    /// If any of the generator names don't exist, it will compose all of the generators before it and return the name of the generator that doesn't exist.
    pub fn compose_generators_into<'a>(
        &self,
        permutation: &mut Permutation,
        generators: impl Iterator<Item = &'a ArcIntern<str>>,
    ) -> Result<(), ArcIntern<str>> {
        for generator in generators {
            let generator = match self.generators.get(&ArcIntern::from(generator.as_ref())) {
                Some(idx) => idx,
                None => return Err(ArcIntern::clone(generator)),
            };

            permutation.compose(generator);
        }

        Ok(())
    }

    /// Find the inverse of an algorithm expressed as a product of generators
    pub fn invert_alg(&self, alg: &mut [ArcIntern<str>]) {
        alg.reverse();

        for generator in alg {
            *generator = ArcIntern::clone(self.generator_inverses.get(generator).unwrap());
        }
    }
}

/// An element of a permutation group
#[derive(Clone)]
pub struct Permutation {
    pub(crate) facelet_count: usize,
    // One of these two must be defined
    mapping: OnceLock<Vec<usize>>,
    cycles: OnceLock<Vec<Vec<usize>>>,
}

impl core::fmt::Display for Permutation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.cycles().iter().fold(String::new(), |mut a, v| {
            a.push('(');
            a.push_str(
                &v.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            );
            a.push(')');

            a
        }))
    }
}

impl core::fmt::Debug for Permutation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Permutation {
    /// Create a permutation using cycles notation. `cycles` is a list of cycles where each cycle is a list of facelet indices.
    pub fn from_cycles(mut cycles: Vec<Vec<usize>>) -> Permutation {
        cycles.retain(|v| v.len() > 1);

        assert!(cycles.iter().all_unique());

        let facelet_count = cycles.iter().flatten().max().map(|v| v + 1).unwrap_or(0);

        Permutation {
            facelet_count,
            mapping: OnceLock::new(),
            cycles: OnceLock::from(cycles),
        }
    }

    /// Get the permutation in mapping notation where `.mapping()[facelet]` gives where the facelet permutes to
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

    fn minimal_mapping(&self) -> &[usize] {
        let mut mapping = self.mapping();

        while mapping.last().copied() == Some(mapping.len() - 1) {
            mapping = &mapping[0..mapping.len() - 1];
        }

        mapping
    }

    /// Get the permutation in cycles notation
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

    /// Find the result of applying the permutation to the identity `power` times.
    ///
    /// This calculates the value in O(1) time with respect to `power`.
    pub fn exponentiate(&mut self, power: Int<I>) {
        self.cycles();
        let mut mapping = self
            .mapping
            .take()
            .unwrap_or_else(|| (0..self.facelet_count).collect_vec());
        let cycles = self.cycles();

        for cycle in cycles {
            let len = Int::<U>::from(cycle.len());
            for i in 0..cycle.len() {
                mapping[cycle[i]] =
                    cycle[TryInto::<usize>::try_into((Int::<I>::from(i) + power) % len).unwrap()];
            }
        }

        self.mapping = OnceLock::from(mapping);
        self.cycles = OnceLock::new();
    }

    fn mapping_mut(&mut self) -> &mut [usize] {
        self.mapping();

        self.mapping.get_mut().unwrap()
    }

    /// Compose another permutation into this permutation
    pub fn compose(&mut self, other: &Permutation) {
        assert_eq!(self.facelet_count, other.facelet_count);

        let my_mapping = self.mapping_mut();
        let other_mapping = other.mapping();

        for value in my_mapping.iter_mut() {
            *value = other_mapping[*value];
        }

        // Invalidate `cycles`
        self.cycles = OnceLock::new();
    }
}

impl PartialEq for Permutation {
    fn eq(&self, other: &Self) -> bool {
        self.minimal_mapping() == other.minimal_mapping()
    }
}

/// A cycle of facelets that is part of the generator of a register
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CycleGeneratorSubcycle {
    pub(crate) facelet_cycle: Vec<usize>,
    pub(crate) chromatic_order: Int<U>,
}

impl CycleGeneratorSubcycle {
    /// Get the cycle of facelets
    pub fn facelet_cycle(&self) -> &[usize] {
        &self.facelet_cycle
    }

    /// Get the order of the cycle after accounting for colors
    pub fn chromatic_order(&self) -> Int<U> {
        self.chromatic_order
    }
}

/// A generator for a register in an architecture
#[derive(Debug, Clone)]
pub struct CycleGenerator {
    pub(crate) generator_sequence: Vec<ArcIntern<str>>,
    pub(crate) permutation: Permutation,
    pub(crate) unshared_cycles: Vec<CycleGeneratorSubcycle>,
    pub(crate) order: Int<U>,
    pub(crate) group: Arc<PermutationGroup>,
}

impl CycleGenerator {
    /// Get the sequence of group generators that compose the cycle generator
    pub fn generator_sequence(&self) -> &[ArcIntern<str>] {
        &self.generator_sequence
    }

    /// Get the underlying permutation of the cycle generator
    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    /// Get the cycles of the permutation that are unshared by other cycles in the architecture
    pub fn unshared_cycles(&self) -> &[CycleGeneratorSubcycle] {
        &self.unshared_cycles
    }

    /// Get the order of the register
    pub fn order(&self) -> Int<U> {
        self.order
    }

    /// Find a collection of facelets that allow decoding the register and that allow determining whether the register is solved
    pub fn signature_facelets(&self) -> Vec<usize> {
        let mut cycles_with_extras = vec![];

        // Create a list of all cycles
        for (i, cycle) in self.unshared_cycles().iter().enumerate() {
            if cycle.chromatic_order() != Int::<U>::one() {
                cycles_with_extras.push((cycle.chromatic_order(), i));
            }
        }

        // Remove all of the cycles that don't contribute to the order of the register, removing the smallest ones first
        cycles_with_extras.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let mut cycles = Vec::<(Int<U>, usize)>::new();

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
            // Find a list of facelets such that for every index in the cycle, at least one facelet is unsolved.
            // On a 3x3, there are only 6 colors, so a subcycle of length 15 will necessarily repeat colors, so if we only include one facelet, the subcycle will appear solved early.
            // TODO: This code doesn't take into account cubies
            let cycle = &self.unshared_cycles()[idx];
            // The chromatic order of a single cycle is bounded by the number of facelets in the permutation group, so this is OK even for big cubes
            let chromatic_order = cycle.chromatic_order().try_into().unwrap();

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
pub struct DecodingTable {
    orders: Vec<Int<U>>,
    table: BTreeMap<Vec<Int<U>>, Vec<ArcIntern<str>>>,
}

impl DecodingTable {
    /// Find the algorithm that creates the requested cycle combination as closely as possible, as a sum of all offsets left over.
    pub fn closest_alg<'s, 't>(
        &'s self,
        target: &'t [Int<U>],
    ) -> (&'s [Int<U>], &'s [ArcIntern<str>]) {
        let mut closest: Option<(Int<U>, &'s [Int<U>], &'s [ArcIntern<str>])> = None;

        let mut update_closest = |achieves: &'s [Int<U>], alg: &'s [ArcIntern<str>]| {
            let dist = achieves
                .iter()
                .copied()
                .zip(target.iter().copied())
                .zip(self.orders.iter().copied())
                .map(|((achieves, target), order)| {
                    let dist = achieves.abs_diff(&target);

                    if dist > order / Int::<U>::from(2_u32) {
                        order - dist
                    } else {
                        dist
                    }
                })
                .sum::<Int<U>>();

            let mut min_dist = dist;

            if match closest {
                Some((old_dist, _, _)) => {
                    min_dist = old_dist;
                    old_dist > dist
                }
                None => true,
            } {
                closest = Some((dist, achieves, alg));
            }

            min_dist
        };

        // Iterate radially away from the closest value lexicographically, hopefully the true closest is nearby

        let mut end_range = self.table.range(target.to_vec()..).chain(self.table.iter());
        let mut take_end = true;
        let mut start_range = self
            .table
            .range(..=target.to_vec())
            .rev()
            .chain(self.table.iter().rev());
        let mut take_start = true;

        let mut amt_taken = 0;

        while (take_end || take_start) && amt_taken < self.table.len() {
            if take_start {
                // Wrapping around should be impossible
                let (achieves, alg) = start_range.next().unwrap();

                amt_taken += 1;

                let min_dist = update_closest(achieves, alg);

                // Taking from here can no longer generate closer values
                if min_dist < target[0].abs_diff(&achieves[0]) {
                    take_start = false;
                }
            }

            if take_end {
                let (achieves, alg) = end_range.next().unwrap();

                amt_taken += 1;

                let min_dist = update_closest(achieves, alg);

                // Taking from here can no longer generate closer values
                if min_dist < achieves[0].abs_diff(&target[0]) {
                    take_end = false;
                }
            }
        }

        let closest = closest.unwrap();

        (closest.1, closest.2)
    }

    pub(crate) fn orders(&self) -> &[Int<U>] {
        &self.orders
    }
}

/// An architecture of a `PermutationGroup`
#[derive(Debug, Clone)]
pub struct Architecture {
    group: Arc<PermutationGroup>,
    cycle_generators: Vec<CycleGenerator>,
    shared_facelets: Vec<usize>,
    optimized_table: Option<Cow<'static, [u8]>>,
    decoded_table: OnceLock<DecodingTable>,
}

impl Architecture {
    /// Create a new architecture from a permutation group and a list of algorithms.
    pub fn new(
        group: Arc<PermutationGroup>,
        algorithms: Vec<Vec<ArcIntern<str>>>,
    ) -> Result<Architecture, ArcIntern<str>> {
        let processed = algorithms_to_cycle_generators(Arc::clone(&group), &algorithms)?;

        Ok(Architecture {
            group,
            cycle_generators: processed.0,
            shared_facelets: processed.1,
            optimized_table: None,
            decoded_table: OnceLock::new(),
        })
    }

    /// Insert a table of optimized algorithms into the architecture. The algorithms are expected to be compressed using `table_encoding::encode`. Inverses and the values that registers that define the architecture need not be optimized, they will be included automatically. You may optimize them anyways and values encoded later in the table will be prioritized.
    ///
    /// `self.get_table()` will panic if the table is encoded incorrectly and it will ignore invalid entries.
    pub fn set_optimized_table(&mut self, optimized_table: Cow<'static, [u8]>) {
        self.optimized_table = Some(optimized_table);
    }

    /// Retrieve a table of optimized algorithms by how they affect each cycle type.
    pub fn get_table(&self) -> &DecodingTable {
        self.decoded_table.get_or_init(|| {
            let table = match &self.optimized_table {
                Some(encoded) => table_encoding::decode_table(encoded).unwrap(),
                None => Vec::new(),
            };

            let registers_decoding_info = self
                .registers()
                .iter()
                .map(|v| {
                    (
                        v.signature_facelets(),
                        PermutePuzzle::new_from_generators(
                            Arc::clone(&self.group),
                            v.generator_sequence().to_owned(),
                        )
                        .unwrap(),
                    )
                })
                .collect_vec();

            let mut data = BTreeMap::new();

            let mut add_permutation = |alg: Vec<ArcIntern<str>>| {
                let permutation =
                    PermutePuzzle::new_from_generators(self.group_arc(), alg.to_owned()).unwrap();

                let maybe_decoded = registers_decoding_info
                    .iter()
                    .map(|reg| decode(permutation.permutation(), &reg.0, &reg.1))
                    .collect::<Option<Vec<_>>>();

                if let Some(decoded) = maybe_decoded {
                    data.insert(decoded, alg);
                }
            };

            for item in self.registers().iter().flat_map(|v| {
                let mut inverse = v.generator_sequence.to_owned();
                self.group.invert_alg(&mut inverse);
                [v.generator_sequence.to_owned(), inverse]
            }) {
                add_permutation(item);
            }

            for item in table.iter().map(|v| {
                let mut inverse = v.to_owned();
                self.group.invert_alg(&mut inverse);
                inverse
            }) {
                add_permutation(item);
            }

            for item in table.into_iter() {
                add_permutation(item);
            }

            DecodingTable {
                table: data,
                orders: self.registers().iter().map(|v| v.order()).collect(),
            }
        })
    }

    /// Get the underlying permutation group
    pub fn group(&self) -> &PermutationGroup {
        &self.group
    }

    /// Get the underlying permutation group as an owned Rc
    pub fn group_arc(&self) -> Arc<PermutationGroup> {
        Arc::clone(&self.group)
    }

    /// Get all of the registers of the architecture
    pub fn registers(&self) -> &[CycleGenerator] {
        &self.cycle_generators
    }

    /// Get all of the facelets that are shared in the architecture
    pub fn shared_facelets(&self) -> &[usize] {
        &self.shared_facelets
    }
}

pub fn puzzle_by_name(name: &str) -> Option<PuzzleDefinition> {
    if name == "3x3" {
        Some(PuzzleDefinition::parse(include_str!("../puzzles/3x3.txt")).unwrap())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use internment::ArcIntern;
    use itertools::Itertools;

    use crate::{Int, I, U};

    use super::{Architecture, PuzzleDefinition};

    #[test]
    fn three_by_three() {
        let cube = PuzzleDefinition::parse(include_str!("../puzzles/3x3.txt")).unwrap();

        for (arch, expected) in [
            (&["U", "D"][..], &[4, 4][..]),
            (
                &["R' F' L U' L U L F U' R", "U F R' D' R2 F R' U' D"],
                &[90_u64, 90],
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
                Arc::clone(&cube.group),
                arch.iter()
                    .map(|v| v.split(" ").map(ArcIntern::from).collect_vec())
                    .collect_vec(),
            )
            .unwrap();

            for (register, expected) in arch.cycle_generators.iter().zip(expected.iter()) {
                assert_eq!(register.order(), Int::<U>::from(*expected));
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
                [ArcIntern::from("U"), ArcIntern::from("L")].iter(),
            )
            .unwrap();

        let mut exp_perm = perm.clone();
        exp_perm.exponentiate(Int::<I>::from(7_u64));

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
