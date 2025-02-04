pub mod from_ksolve;
pub mod puzzle;

#[cfg(test)]
mod tests {
    // use super::*;

    // use cube::CubeState;
    // use pruning::PruningTables;
    // use solver::IDASolver;
    // use std::time::Instant;

    // #[test]
    // fn test_cycle_type() {
    //     let cycle_type = CycleType {
    //         corner_partition: vec![(3, true), (5, true)],
    //         edge_partition: vec![(2, true), (2, true)],
    //     };

    //     let mut tag = "corners".to_string();
    //     for &(corner, orient) in cycle_type.corner_partition.iter() {
    //         tag.push_str(&format!("{}{}", corner, if orient { "o" } else { "n" }));
    //     }
    //     let pruning_tables = PruningTables::from(&tag, &cycle_type);
    //     let now = Instant::now();
    //     let mut solver = IDASolver::new(CubeState::default(), &pruning_tables, cycle_type);
    //     let solution = solver.solve();
    //     let elapsed = now.elapsed();
    //     println!("{}", solution);
    //     println!("Found phase 2 solution in {:.2?}", elapsed);
    // }
}
