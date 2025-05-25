use std::{collections::VecDeque, option::Option, sync::Arc};

use itertools::Itertools;

use crate::architectures::{Algorithm, Permutation, PermutationGroup};

use super::{I, Int, U};

pub struct BeginnerMethod {
    group: Arc<PermutationGroup>,
    stabilizers: Stabilizer,
}

impl BeginnerMethod {
    #[must_use]
    #[expect(clippy::missing_panics_doc)]
    pub fn new(group: Arc<PermutationGroup>) -> BeginnerMethod {
        let mut stabilizers = Stabilizer::new(
            Arc::clone(&group),
            &(0..group.facelet_count()).collect_vec(),
        );

        for (name, _) in group.generators() {
            stabilizers
                .extend(Algorithm::new_from_move_seq(Arc::clone(&group), vec![name]).unwrap());
        }

        BeginnerMethod { group, stabilizers }
    }

    pub fn solve(&self, permutation: Permutation) -> Option<Algorithm> {
        let mut alg = Algorithm::identity(Arc::clone(&self.group));
        if self.stabilizers.solve(permutation, Some(&mut alg)) {
            Some(alg)
        } else {
            None
        }
    }

    pub fn is_member(&self, permutation: Permutation) -> bool {
        self.stabilizers.solve(permutation, None)
    }

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
    generating_set: Vec<Algorithm>,
    coset_reps: Box<[Option<Algorithm>]>,
}

impl Stabilizer {
    fn new(group: Arc<PermutationGroup>, chain: &[usize]) -> Stabilizer {
        let (head, tail) = chain.split_first().unwrap();

        let mut coset_reps = Box::<[_]>::from(vec![None; group.facelet_count()]);
        coset_reps[*head] = Some(Algorithm::identity(Arc::clone(&group)));

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
    fn solve(&self, mut permutation: Permutation, mut maybe_alg: Option<&mut Algorithm>) -> bool {
        // println!("{} — {}", self.stabilizes, permutation);
        loop {
            let rep = permutation.mapping()[self.stabilizes];

            if rep == self.stabilizes {
                break;
            }

            let Some(other_alg) = &self.coset_reps[rep] else {
                return false;
            };

            if let Some(alg) = maybe_alg.as_mut() {
                alg.compose_into(other_alg);
            }
            permutation.compose_into(other_alg.permutation());
        }

        match &self.next {
            Some(next) => next.solve(permutation, maybe_alg),
            None => true,
        }
    }

    fn inverse_rep_to(&self, mut rep: usize, alg: &mut Algorithm) -> Result<(), ()> {
        while rep != self.stabilizes {
            let Some(other_alg) = &self.coset_reps[rep] else {
                return Err(());
            };

            alg.compose_into(other_alg);
            rep = other_alg.permutation().mapping()[rep];
        }

        Ok(())
    }

    fn extend(&mut self, generator: Algorithm) {
        if self.solve(generator.permutation().to_owned(), None) {
            // TODO: Check if the generator is shorter than the ones we already have
            return;
        }
        // println!("{} {generator:?}", self.stabilizes);

        self.generating_set.push(generator);
        let generator = self.generating_set.last().unwrap();

        let mapping = generator.permutation().mapping().to_owned();
        let mut inv = generator.clone();
        inv.exponentiate(-Int::<I>::one());

        // TODO: Some kind of SSSP thing to make these coset reps as short as possible
        let mut newly_in_orbit = VecDeque::new();

        #[expect(clippy::needless_range_loop)] // false positive
        for i in 0..self.coset_reps.len() {
            if self.coset_reps[i].is_some() && self.coset_reps[mapping[i]].is_none() {
                self.coset_reps[mapping[i]] = Some(inv.clone());
                newly_in_orbit.push_back(mapping[i]);
            }
        }

        while let Some(spot) = newly_in_orbit.pop_front() {
            for alg in &self.generating_set {
                let goes_to = alg.permutation().mapping()[spot];
                if self.coset_reps[goes_to].is_none() {
                    let mut inv_alg = alg.clone();
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
            let mut rep = Algorithm::identity(Arc::clone(&self.group));
            let Ok(()) = self.inverse_rep_to(i, &mut rep) else {
                continue;
            };

            rep.exponentiate(-Int::<I>::one());

            for generator in &self.generating_set {
                let mut new_generator = rep.clone();
                new_generator.compose_into(generator);
                self.inverse_rep_to(
                    new_generator.permutation().mapping()[self.stabilizes],
                    &mut new_generator,
                )
                .unwrap();
                self.next.as_mut().unwrap().extend(new_generator);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use internment::ArcIntern;

    use crate::{
        Int, U,
        architectures::{Algorithm, Permutation, PermutationGroup, PuzzleDefinition},
    };

    use super::BeginnerMethod;

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
        ));

        let method = BeginnerMethod::new(Arc::clone(&puzzle));
        assert_eq!(method.cardinality(), Int::<U>::from(3_u32));
        assert!(!method.is_member(Permutation::from_cycles(vec![vec![0, 1]])));
        assert!(method.is_member(Permutation::from_cycles(vec![vec![0, 1, 2]])));
        assert_eq!(
            method.solve(Permutation::from_cycles(vec![vec![0, 1, 2]])),
            Some(Algorithm::new_from_move_seq(puzzle, vec![ArcIntern::from("B")]).unwrap())
        );
    }

    #[test]
    fn three_by_three() {
        let cube_def = PuzzleDefinition::parse(include_str!("../../puzzles/3x3.txt"))
            .unwrap()
            .perm_group;

        let method = BeginnerMethod::new(Arc::clone(&cube_def));

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
