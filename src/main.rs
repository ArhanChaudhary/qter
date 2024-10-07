use rusty_rubik::{
    cube::{get_index_of_state, CubeState, MoveSequence},
    parser::parse_scramble, pruning::PruningTables, solver::{IDASolver, Solver},
};
// let (c, eo, ep) = get_index_of_state(&CubeState::default());
//         assert_eq!(c, 0);
//         assert_eq!(eo, 0);
//         assert_eq!(ep, 0);

fn main() {
    let tables = PruningTables::default_tables();
    let scramble = MoveSequence(parse_scramble("F L D D D F F F U U U R U U U B B B R B D R R D D D L F F F U U F F F L U U U F U U L L F F F L F U").unwrap());
    let solved = CubeState::default();
    let twisted = solved.apply_move_instances(&scramble);
    let solver = IDASolver::new(twisted, &tables);
    let solution = solver.solve();
    println!("Solution: {:?}", solution.get_moves());
    return;
    loop {
        // interpreter loop
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        if input == "exit" {
            break;
        }
        let Ok(scramble) = parse_scramble(input) else {
            continue;
        };
        let scramble = MoveSequence(scramble);
        let solved = CubeState::default();
        let twisted = solved.apply_move_instances(&scramble);
        println!("Twisted state: {:?}", twisted);
        // let num = index_of_state(&twisted);
        // println!("Index of state: {:#064b}", num);
        // let tables = PruningTables::default_tables();
        // let solver = IDASolver::new(twisted, &tables);
        // use std::time::Instant;
        // let now = Instant::now();
        // let solution = solver.solve();
        // println!("Solution: {:?}", solution.get_moves());
        // let elapsed = now.elapsed();
        // println!("Elapsed: {:.2?}", elapsed);
    }
}
