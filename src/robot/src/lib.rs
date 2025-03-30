use std::collections::{HashMap, HashSet};

use internment::ArcIntern;
use itertools::Itertools;
use qter_core::architectures::{Permutation, PermutationGroup};
use rand::{Rng, seq::IndexedRandom};

// TODO: For arbitrary puzzles, make this work so that it allows missing colors that are outside of a facelet's orbit

/// Represents an equivalence class of facelets that have the same pattern
#[derive(Debug)]
pub struct IdenticalPatternEquivalenceClass {
    /// The facelets that have the same pattern
    pub facelets: Vec<usize>,
    /// The pattern that they share
    ///
    /// Note that they can share pattern up to color renaming, for example `red → blue` has the same pattern as `green → orange`: [0, 1].
    pub pattern_shared: Vec<usize>,
}

/// If an algorithm does not work as a calibration algorithm, this will be returned detailing why.
#[derive(Debug)] // TODO: Display?
pub struct CalibrationAlgFailures {
    /// Each facelet index in the list did not see the listed colors during the calibration alg
    pub facelet_does_not_see_all_colors: Vec<(usize, Vec<ArcIntern<str>>)>,
    /// Each group of facelets in each element of the list have identical patterns throughout the alg (up to color renaming) and would be indistinguishable by the computer vision algorithm
    pub identical_patterns: Vec<IdenticalPatternEquivalenceClass>,
    /// Set to `true` if the calibration algorithm does not resolve the cube
    pub does_not_resolve: bool,
}

pub fn validate_calibration_alg(
    group: &PermutationGroup,
    alg: &[ArcIntern<str>],
) -> Option<CalibrationAlgFailures> {
    let mut puzzle = group.identity();

    let mut unique_colors_found = vec![HashSet::new(); puzzle.mapping().len()];

    let mut patterns = vec![(HashMap::new(), Vec::new()); puzzle.mapping().len()];

    let color_set = group
        .facelet_colors()
        .iter()
        .cloned()
        .collect::<HashSet<_>>();

    let mut process_position = |puzzle: &Permutation| {
        for ((colors_found, &maps_to), (mapping, pattern)) in unique_colors_found
            .iter_mut()
            .zip(puzzle.mapping())
            .zip(patterns.iter_mut())
        {
            let color = group.facelet_colors()[maps_to].to_owned();

            let len = mapping.len();
            let code = *mapping.entry(color.to_owned()).or_insert(len);
            pattern.push(code);

            colors_found.insert(color);
        }
    };

    process_position(&puzzle);

    for generator in alg {
        puzzle.compose(group.get_generator(generator).unwrap());

        process_position(&puzzle);
    }

    let mut failures = CalibrationAlgFailures {
        facelet_does_not_see_all_colors: Vec::new(),
        identical_patterns: Vec::new(),
        does_not_resolve: false,
    };

    if puzzle != group.identity() {
        failures.does_not_resolve = true;
    }

    failures.identical_patterns = patterns
        .into_iter()
        .map(|(_, pattern)| pattern)
        .enumerate()
        .sorted_by(|a, b| a.1.cmp(&b.1))
        .map(|(i, pattern)| IdenticalPatternEquivalenceClass {
            facelets: vec![i],
            pattern_shared: pattern,
        })
        .coalesce(|mut a, b| {
            if a.pattern_shared == b.pattern_shared {
                a.facelets.extend_from_slice(&b.facelets);
                Ok(a)
            } else {
                Err((a, b))
            }
        })
        .filter(|v| v.facelets.len() > 1)
        .collect_vec();

    for (i, set) in unique_colors_found.iter().enumerate() {
        if set.len() < color_set.len() {
            failures
                .facelet_does_not_see_all_colors
                .push((i, color_set.difference(set).cloned().collect()))
        }
    }

    if failures.does_not_resolve
        || !failures.identical_patterns.is_empty()
        || !failures.facelet_does_not_see_all_colors.is_empty()
    {
        Some(failures)
    } else {
        None
    }
}

/// Find _any_ valid calibration scramble for the given puzzle. Note that this does not attempt to find a _shortest_ calibration scramble and it does not include a descrambling/solving step.
pub fn find_calibration_scramble(
    group: &PermutationGroup,
    rng: &mut impl Rng,
) -> Vec<ArcIntern<str>> {
    let mut alg = Vec::new();

    let generators = group
        .generators()
        .map(|v| v.0)
        .sorted_unstable()
        .collect_vec();

    let mut affected_by = vec![Vec::new(); group.facelet_count()];

    for (generator, perm) in group.generators() {
        for &facelet in perm.cycles().iter().flat_map(|cycle| cycle.iter()) {
            affected_by[facelet].push(generator.to_owned());
        }
    }

    for item in affected_by.iter_mut() {
        item.sort_unstable();
    }

    loop {
        let err = validate_calibration_alg(group, &alg);

        // TODO: Make this code smarter

        let err = match err {
            Some(err) => err,
            None => return alg,
        };

        if err.identical_patterns.is_empty() && err.facelet_does_not_see_all_colors.is_empty() {
            break;
        }

        if rng.random() {
            alg.push(generators.choose(rng).unwrap().to_owned());
        } else {
            let mut problem_facelets = vec![false; group.facelet_count()];

            for facelet in err
                .identical_patterns
                .iter()
                .flat_map(|v| &v.facelets)
                .copied()
                .chain(err.facelet_does_not_see_all_colors.iter().map(|v| v.0))
            {
                problem_facelets[facelet] = true;
            }

            let list_of_problem_facelets = problem_facelets
                .iter()
                .enumerate()
                .filter(|v| *v.1)
                .map(|v| v.0)
                .collect_vec();

            let facelet = list_of_problem_facelets.choose(rng).unwrap();

            alg.push(affected_by[*facelet].choose(rng).unwrap().to_owned());
        }
    }

    alg
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use internment::ArcIntern;
    use itertools::Itertools;
    use qter_core::architectures::{PermutationGroup, PuzzleDefinition};
    use rand::{SeedableRng, rngs::StdRng};

    use crate::{find_calibration_scramble, validate_calibration_alg};

    fn three_by_three() -> Arc<PermutationGroup> {
        let cube =
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap();

        cube.perm_group
    }

    #[test]
    fn no_alg() {
        let err = validate_calibration_alg(&three_by_three(), &[]).unwrap();

        assert!(!err.does_not_resolve);

        assert_eq!(err.facelet_does_not_see_all_colors.len(), 48);
        assert_eq!(
            err.facelet_does_not_see_all_colors
                .iter()
                .map(|v| v.0)
                .collect::<HashSet<_>>()
                .len(),
            48
        );
        for item in err.facelet_does_not_see_all_colors {
            assert_eq!(item.1.len(), 5);
        }

        assert_eq!(err.identical_patterns.len(), 1);
        assert_eq!(err.identical_patterns[0].pattern_shared, vec![0]);
        assert_eq!(
            err.identical_patterns[0]
                .facelets
                .iter()
                .collect::<HashSet<_>>()
                .len(),
            48
        );
    }

    #[test]
    fn simple_alg() {
        let err = validate_calibration_alg(&three_by_three(), &[ArcIntern::from("R")]).unwrap();

        assert!(err.does_not_resolve);

        assert_eq!(err.facelet_does_not_see_all_colors.len(), 48);
        assert_eq!(
            err.facelet_does_not_see_all_colors
                .iter()
                .map(|v| v.0)
                .collect::<HashSet<_>>()
                .len(),
            48
        );
        let mut five = 0;
        let mut four = 0;
        for item in err.facelet_does_not_see_all_colors {
            if item.1.len() == 5 {
                five += 1;
            } else if item.1.len() == 4 {
                four += 1;
            } else {
                panic!()
            }
        }
        assert_eq!(five, 36);
        assert_eq!(four, 12);

        assert_eq!(err.identical_patterns.len(), 2);
        let unmoved_pattern = err
            .identical_patterns
            .iter()
            .find(|v| v.pattern_shared == vec![0, 0])
            .unwrap();
        assert_eq!(
            unmoved_pattern
                .facelets
                .iter()
                .collect::<HashSet<_>>()
                .len(),
            36
        );

        let moved_once_pattern = err
            .identical_patterns
            .iter()
            .find(|v| v.pattern_shared == vec![0, 1])
            .unwrap();
        assert_eq!(
            moved_once_pattern
                .facelets
                .iter()
                .collect::<HashSet<_>>()
                .len(),
            12
        );
    }

    // #[test]
    fn short_one() {
        let group = three_by_three();

        let mut shortest_len = usize::MAX;
        let mut shortest_i = 0;

        for i in 0..25 {
            let mut seed = *b"They call me... THE SWIZZLER!!!!";
            seed[31] = i;

            let alg = find_calibration_scramble(&group, &mut StdRng::from_seed(seed));

            if alg.len() < shortest_len {
                shortest_len = alg.len();
                shortest_i = i;
            }
        }

        panic!("{shortest_i}, {}", shortest_len);
    }

    #[test]
    fn good_alg() {
        let group = three_by_three();

        let mut alg = find_calibration_scramble(
            &group,
            &mut StdRng::from_seed(*b"They call me... THE SWIZZLER!!!\x13"),
        );

        let mut descramble = alg.to_owned();
        group.invert_generator_moves(&mut descramble);
        alg.extend_from_slice(&descramble);

        let err = validate_calibration_alg(&group, &alg);

        if let Some(err) = err {
            panic!("{err:?}");
        }
    }

    #[test]
    fn good_alg2() {
        let alg = "L2 U2 B D2 L R D D2 B F' U D D U' D R' U L D' U' D2 F2 U2 R2 U2 D' U U F' L2 F' F' L D2 F' D' B B D D U' L' R R' D' B2 L2 F D' B' L2 F2 B' D2 B2 R' L2 F' B2 U L B' R' R2 F' D' R2 R B R' D' B' R' U2 B L2 R' B2 R2 D B' L2 F2 D2 L D R U' B R2 R2 R B' F' D2 D' D L2 F' F R' D R' U2 L2 R' D U' R' F' U2 F' D' R2 U L R2";

        let err = validate_calibration_alg(
            &three_by_three(),
            &alg.split_whitespace().map(ArcIntern::from).collect_vec(),
        );

        if let Some(err) = err {
            panic!("{err:?}");
        }
    }
}
