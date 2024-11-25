use std::{cell::RefCell, collections::HashSet, rc::Rc};

use bnum::types::U512;

use crate::{
    architectures::{CycleGenerator, CycleGeneratorSubcycle, Permutation, PermutationGroup},
    discrete_math::lcm,
};

#[derive(Debug)]
enum UnionFindEntry {
    RootOfSet {
        // For weighted union-find
        weight: usize,
        // Which algorithms share the facelets in this set?
        contains_facelets_from: HashSet<usize>,
    },
    OwnedBy(RefCell<usize>),
}

#[derive(Debug)]
struct UnionFindOfCycles {
    sets: Vec<UnionFindEntry>,
}

impl UnionFindOfCycles {
    /// Returns the index of the root entry representing the facelet's orbit as well as the number of facelets and the set of algorithms that contribute to the facelet's orbit
    fn find(&self, facelet: usize) -> (usize, (usize, &HashSet<usize>)) {
        match &self.sets[facelet] {
            UnionFindEntry::RootOfSet {
                weight,
                contains_facelets_from,
            } => (facelet, (*weight, contains_facelets_from)),
            UnionFindEntry::OwnedBy(parent_idx) => {
                let ret = self.find(*parent_idx.borrow());
                *(parent_idx.borrow_mut()) = ret.0;
                ret
            }
        }
    }

    fn union(&mut self, a: usize, b: usize) {
        let (root_a, (weight_a, sets_a)) = self.find(a);
        let (root_b, (weight_b, sets_b)) = self.find(b);

        let mut combined_sets = sets_a.to_owned();
        combined_sets.extend(sets_b);

        if root_a == root_b {
            return;
        }

        if weight_a > weight_b {
            match &mut self.sets[root_a] {
                UnionFindEntry::RootOfSet {
                    weight,
                    contains_facelets_from,
                } => {
                    *weight += weight_b;
                    *contains_facelets_from = combined_sets;
                }
                UnionFindEntry::OwnedBy(_) => unreachable!(),
            }

            self.sets[root_b] = UnionFindEntry::OwnedBy(RefCell::new(root_a));
        } else {
            match &mut self.sets[root_b] {
                UnionFindEntry::RootOfSet {
                    weight,
                    contains_facelets_from,
                } => {
                    *weight += weight_a;
                    *contains_facelets_from = combined_sets;
                }
                UnionFindEntry::OwnedBy(_) => unreachable!(),
            }

            self.sets[root_a] = UnionFindEntry::OwnedBy(RefCell::new(root_b));
        }
    }

    /// Calculate the orbits of all of the facelets along with which algorithms contribute to the orbit
    fn find_orbits(facelet_count: usize, permutations: &[Permutation]) -> UnionFindOfCycles {
        let mut sets = vec![];

        for facelet in 0..facelet_count {
            let mut contains_facelets_from = HashSet::new();

            for (i, permutation) in permutations.iter().enumerate() {
                if permutation.mapping()[facelet] != facelet {
                    contains_facelets_from.insert(i);
                }
            }

            sets.push(UnionFindEntry::RootOfSet {
                weight: 1,
                contains_facelets_from,
            })
        }

        let mut union_find = UnionFindOfCycles { sets };

        for permutation in permutations {
            for facelet in 0..facelet_count {
                let goes_to = permutation.mapping()[facelet];

                // They have the same orbit
                union_find.union(facelet, goes_to);
            }
        }

        union_find
    }
}

fn length_of_substring_that_this_string_is_n_repeated_copies_of<'a>(
    colors: impl Iterator<Item = &'a str>,
) -> U512 {
    let mut found = vec![];
    let mut current_repeat_length = 1;

    for (i, color) in colors.enumerate() {
        found.push(color);

        if found[i % current_repeat_length] != color {
            current_repeat_length = i + 1;
        }
    }

    // We didn't match the substring a whole number of times; it actually doesn't work
    if found.len() % current_repeat_length != 0 {
        current_repeat_length = found.len();
    }

    U512::from_digit(current_repeat_length as u64)
}

pub fn algorithms_to_cycle_generators(
    group: Rc<PermutationGroup>,
    algorithms: &[Vec<String>],
) -> Result<(Vec<CycleGenerator>, Vec<usize>), String> {
    let mut permutations = vec![];

    for algorithm in algorithms {
        let mut permutation = group.identity();
        group.compose_generators_into(&mut permutation, algorithm)?;
        permutations.push(permutation);
    }

    let orbits = UnionFindOfCycles::find_orbits(group.facelet_count(), &permutations);

    let mut shared_facelets = vec![];

    Ok((
        permutations
            .into_iter()
            .zip(algorithms.iter())
            .map(|(permutation, algorithm)| {
                let mut unshared_cycles = vec![];

                for cycle in permutation.cycles() {
                    if orbits.find(cycle[0]).1 .1.len() > 1 {
                        shared_facelets.extend_from_slice(cycle);
                        continue;
                    }

                    let chromatic_order =
                        length_of_substring_that_this_string_is_n_repeated_copies_of(
                            cycle.iter().map(|v| &**group.facelet_colors()[*v]),
                        );

                    unshared_cycles.push(CycleGeneratorSubcycle {
                        facelet_cycle: cycle.to_owned(),
                        chromatic_order,
                    });
                }

                CycleGenerator {
                    generator_sequence: algorithm.to_owned(),
                    permutation,
                    order: unshared_cycles
                        .iter()
                        .fold(U512::ONE, |a, v| lcm(a, v.chromatic_order)),
                    unshared_cycles,
                    group: Rc::clone(&group),
                }
            })
            .collect(),
        shared_facelets,
    ))
}

#[cfg(test)]
mod tests {
    use bnum::types::U512;

    use crate::architectures::{CycleGeneratorSubcycle, PuzzleDefinition};

    use super::length_of_substring_that_this_string_is_n_repeated_copies_of;

    #[test]
    fn length_of_substring_whatever() {
        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "a", "a", "a"].into_iter()
            )
            .digits()[0],
            1
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "a", "b"].into_iter()
            )
            .digits()[0],
            2
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "a", "b", "a"].into_iter()
            )
            .digits()[0],
            5
        );

        assert_eq!(
            length_of_substring_that_this_string_is_n_repeated_copies_of(
                ["a", "b", "c", "d", "e"].into_iter()
            )
            .digits()[0],
            5
        );
    }

    #[test]
    fn simple() {
        let PuzzleDefinition { group: _, presets } = PuzzleDefinition::parse(
            "
                COLORS

                A -> 1
                B -> 2
                C -> 3
                D -> 4
                E -> 5
                F -> 6
                G -> 7
                H -> 8
                I -> 9
                J -> 10
                K -> 11, 12, 13

                GENERATORS

                A = (1, 2, 3)
                B = (4, 5, 6)
                C = (6, 7, 8)
                D = (9, 10)
                E = (11, 12, 13)

                DERIVED

                PRESETS

                (3, 2) A B / C D E
            ",
        )
        .unwrap();

        let preset = &presets[0];

        for i in 3..=7 {
            assert!(preset.shared_facelets().contains(&i));
        }

        assert_eq!(preset.registers()[0].order, U512::from_digit(3));
        assert_eq!(
            preset.registers()[0].unshared_cycles,
            vec![CycleGeneratorSubcycle {
                facelet_cycle: vec![0, 1, 2],
                chromatic_order: U512::from_digit(3),
            }]
        );
        assert_eq!(preset.registers()[1].order, U512::from_digit(2));
        assert_eq!(
            preset.registers()[1].unshared_cycles,
            vec![
                CycleGeneratorSubcycle {
                    facelet_cycle: vec![8, 9],
                    chromatic_order: U512::from_digit(2)
                },
                CycleGeneratorSubcycle {
                    facelet_cycle: vec![10, 11, 12],
                    chromatic_order: U512::ONE,
                }
            ]
        );
    }
}
