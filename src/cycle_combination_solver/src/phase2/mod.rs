//! A crate used to model the Rubik's Cube.
//!
//! The crate includes consists of two separate compartments:
//!
//! - An **executable** that allows you to instantly search for a solution to a
//!   configuration of the Rubik's Cube.
//!
//! - A **library** that provides utility functions for solver methods, pruning table
//!   generation, and an API for Rubik's Cube structure.
//!
//!

pub mod cube;
pub mod parser;
pub mod pruning;
pub mod puzzle;
pub mod solver;

#[derive(Default)]
pub struct CycleType<T> {
    pub corner_partition: Vec<(T, bool)>,
    pub edge_partition: Vec<(T, bool)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use cube::CubeState;
    use pruning::PruningTables;
    use solver::IDASolver;
    use std::time::Instant;

    #[cfg(feature = "slow-tests")]
    #[test]
    fn test_cycle_type() {
        let cycle_type = CycleType {
            corner_partition: vec![(3, true), (5, true)],
            edge_partition: vec![(2, true), (2, true)],
        };

        let mut tag = "corners".to_string();
        for &(corner, orient) in cycle_type.corner_partition.iter() {
            tag.push_str(&format!("{}{}", corner, if orient { "o" } else { "n" }));
        }
        let pruning_tables = PruningTables::from(&tag, &cycle_type);
        let now = Instant::now();
        let mut solver = IDASolver::new(CubeState::default(), &pruning_tables, cycle_type);
        let solution = solver.solve();
        let elapsed = now.elapsed();
        println!("{}", solution);
        println!("Found phase 2 solution in {:.2?}", elapsed);
    }
}
