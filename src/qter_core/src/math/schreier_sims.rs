use std::{collections::VecDeque, option::Option, sync::Arc};

use itertools::Itertools;

use crate::architectures::{Permutation, PermutationGroup};

use super::{I, Int, U};

pub struct StabilizerChain {
    stabilizers: Stabilizer,
}

impl StabilizerChain {
    /// Create a new stabilizer chain from the permutation group using the Schreier-Sims algorithm.
    #[must_use]
    pub fn new(group: &Arc<PermutationGroup>) -> StabilizerChain {
        let mut stabilizers =
            Stabilizer::new(Arc::clone(group), &(0..group.facelet_count()).collect_vec());

        for (_, perm) in group.generators() {
            stabilizers.extend(perm.to_owned());
        }

        StabilizerChain { stabilizers }
    }

    /// Determine if a permutation is a member of the group
    #[must_use]
    pub fn is_member(&self, permutation: Permutation) -> bool {
        self.stabilizers.is_member(permutation)
    }

    /// Calculate the cardinality of the group
    #[must_use]
    pub fn cardinality(&self) -> Int<U> {
        self.stabilizers.cardinality()
    }
}

#[derive(Debug)]
struct Stabilizer {
    group: Arc<PermutationGroup>,
    next: Option<Box<Stabilizer>>,
    stabilizes: usize,
    generating_set: Vec<Permutation>,
    coset_reps: Box<[Option<Permutation>]>,
}

impl Stabilizer {
    fn new(group: Arc<PermutationGroup>, chain: &[usize]) -> Stabilizer {
        let (head, tail) = chain.split_first().unwrap();

        let mut coset_reps = Box::<[_]>::from(vec![None; group.facelet_count()]);
        coset_reps[*head] = Some(group.identity());

        Stabilizer {
            stabilizes: *head,
            next: (!tail.is_empty()).then(|| Box::new(Stabilizer::new(Arc::clone(&group), tail))),
            coset_reps,
            generating_set: Vec::new(),
            group,
        }
    }

    fn cardinality(&self) -> Int<U> {
        let mut cardinality = Int::from(self.coset_reps.iter().filter(|v| v.is_some()).count());
        if let Some(next) = &self.next {
            cardinality *= next.cardinality();
        }
        cardinality
    }

    #[must_use]
    fn is_member(&self, mut permutation: Permutation) -> bool {
        // println!("{} — {}", self.stabilizes, permutation);
        loop {
            let rep = permutation
                .mapping()
                .get(self.stabilizes)
                .copied()
                .unwrap_or(self.stabilizes);

            if rep == self.stabilizes {
                break;
            }

            let Some(other_perm) = &self.coset_reps[rep] else {
                return false;
            };

            permutation.compose_into(other_perm);
        }

        match &self.next {
            Some(next) => next.is_member(permutation),
            None => true,
        }
    }

    fn inverse_rep_to(&self, mut rep: usize, alg: &mut Permutation) -> Result<(), ()> {
        while rep != self.stabilizes {
            let Some(other_alg) = &self.coset_reps[rep] else {
                return Err(());
            };

            alg.compose_into(other_alg);
            rep = other_alg.mapping()[rep];
        }

        Ok(())
    }

    fn extend(&mut self, generator: Permutation) {
        if self.is_member(generator.clone()) {
            // TODO: Check if the generator is shorter than the ones we already have
            return;
        }
        // println!("{} {generator:?}", self.stabilizes);

        self.generating_set.push(generator);
        let generator = self.generating_set.last().unwrap();

        let mapping = generator.mapping().to_owned();
        let mut inv = generator.clone();
        inv.exponentiate(-Int::<I>::one());

        // TODO: Some kind of SSSP thing to make these coset reps as short as possible
        let mut newly_in_orbit = VecDeque::new();

        for i in 0..self.coset_reps.len() {
            if self.coset_reps[i].is_some()
                && self.coset_reps[mapping.get(i).copied().unwrap_or(i)].is_none()
            {
                self.coset_reps[mapping[i]] = Some(inv.clone());
                newly_in_orbit.push_back(mapping[i]);
            }
        }

        while let Some(spot) = newly_in_orbit.pop_front() {
            for perm in &self.generating_set {
                let goes_to = perm.mapping().get(spot).copied().unwrap_or(spot);
                if self.coset_reps[goes_to].is_none() {
                    let mut inv_alg = perm.clone();
                    inv_alg.exponentiate(-Int::<I>::one());
                    self.coset_reps[goes_to] = Some(inv_alg);
                    newly_in_orbit.push_back(goes_to);
                }
            }
        }

        if self.next.is_none() {
            return;
        }

        for i in 0..self.coset_reps.len() {
            let mut rep = self.group.identity();
            let Ok(()) = self.inverse_rep_to(i, &mut rep) else {
                continue;
            };

            rep.exponentiate(-Int::<I>::one());

            for generator in &self.generating_set {
                let mut new_generator = rep.clone();
                new_generator.compose_into(generator);
                self.inverse_rep_to(new_generator.mapping()[self.stabilizes], &mut new_generator)
                    .unwrap();
                self.next.as_mut().unwrap().extend(new_generator);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use chumsky::Parser;
    use internment::ArcIntern;

    use crate::{
        File, Int, Span, U,
        architectures::{Algorithm, Permutation, PermutationGroup, puzzle_definition},
    };

    use super::StabilizerChain;

    #[test]
    fn simple() {
        let mut perms = HashMap::new();
        perms.insert(
            ArcIntern::from("A"),
            Permutation::from_cycles(vec![vec![0, 1, 2]]),
        );
        perms.insert(
            ArcIntern::from("B"),
            Permutation::from_cycles(vec![vec![0, 2, 1]]),
        );

        let puzzle = Arc::new(PermutationGroup::new(
            vec![
                ArcIntern::from("a"),
                ArcIntern::from("b"),
                ArcIntern::from("c"),
            ],
            perms,
            Span::from_static("thingy"),
        ));

        let method = StabilizerChain::new(&puzzle);
        assert_eq!(method.cardinality(), Int::<U>::from(3_u32));
        assert!(!method.is_member(Permutation::from_cycles(vec![vec![0, 1]])));
        assert!(method.is_member(Permutation::from_cycles(vec![vec![0, 1, 2]])));
    }

    #[test]
    fn three_by_three() {
        let cube_def = Arc::clone(
            &puzzle_definition()
                .parse(File::from("3x3"))
                .unwrap()
                .perm_group,
        );

        let method = StabilizerChain::new(&cube_def);

        assert_eq!(
            method.cardinality(),
            "43252003274489856000".parse::<Int<U>>().unwrap()
        );

        // Corner twist
        assert!(!method.is_member(Permutation::from_cycles(vec![vec![10, 16, 5]])));

        // U alg
        assert!(
            method.is_member(
                Algorithm::new_from_move_seq(Arc::clone(&cube_def), vec![ArcIntern::from("U")])
                    .unwrap()
                    .permutation()
                    .clone()
            )
        );

        // Sexy move
        assert!(
            method.is_member(
                Algorithm::new_from_move_seq(
                    Arc::clone(&cube_def),
                    vec![
                        ArcIntern::from("U"),
                        ArcIntern::from("R"),
                        ArcIntern::from("U'"),
                        ArcIntern::from("R'"),
                    ]
                )
                .unwrap()
                .permutation()
                .clone()
            )
        );

        // Two corner twists to make the orientation sum work
        assert!(method.is_member(Permutation::from_cycles(vec![
            vec![10, 16, 5],
            vec![18, 7, 24]
        ])));
    }
}
