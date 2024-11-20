use std::{cell::RefCell, collections::HashSet};

use bnum::types::U512;

use crate::architectures::{CycleGenerator, Permutation, PermutationGroup};

enum UnionFindEntry {
    RootOfSet {
        // For weighted union-find
        weight: usize,
        // Which algorithms share the facelets in this set?
        contains_facelets_from: HashSet<usize>,
    },
    OwnedBy(RefCell<usize>),
}

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

fn gcd(mut a: U512, mut b: U512) -> U512 {
    loop {
        if b == U512::ONE {
            return a;
        }

        let rem = a.rem_euclid(b);
        a = b;
        b = rem;
    }
}

fn lcm(a: U512, b: U512) -> U512 {
    a / gcd(a, b) * b
}

pub fn algorithms_to_cycle_generators(
    group: &PermutationGroup,
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

                    unshared_cycles.push(cycle.to_owned());
                }

                CycleGenerator {
                    generator_sequence: algorithm.to_owned(),
                    permutation,
                    order: unshared_cycles
                        .iter()
                        .fold(U512::ONE, |a, v| lcm(a, U512::from_digit(v.len() as u64))),
                    unshared_cycles,
                }
            })
            .collect(),
        shared_facelets,
    ))
}
