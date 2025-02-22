use cycle_combination_solver::phase2::{
    pruning::ZeroTable,
    puzzle::cube3::Cube3,
    solver::CycleTypeSolver,
};
use puzzle_geometry::ksolve::KPUZZLE_3X3;

fn main() {
    // TODO: make this a test case
    let solver: CycleTypeSolver<Cube3, _, [Cube3; 21]> = CycleTypeSolver::new(
        (&*KPUZZLE_3X3).try_into().unwrap(),
        vec![
            vec![(1.try_into().unwrap(), true), (5.try_into().unwrap(), true)],
            vec![(1.try_into().unwrap(), true), (7.try_into().unwrap(), true)],
        ],
        ZeroTable,
    );
    let solutions = solver.solve();
    for solution in solutions {
        for move_ in solution.iter() {
            print!("{} ", move_.name);
        }
        println!();
    }
}
