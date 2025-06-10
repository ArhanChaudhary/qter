pub mod canonical_fsm;
pub mod orbit_puzzle;
pub mod permutator;
pub mod pruning;
pub mod puzzle;
pub mod puzzle_state_history;
pub mod solver;
pub use generativity;

// We can do one more however it will overflow when adding more to it which is
// common in context
const FACT_UNTIL_19: [u64; 20] = {
    let mut arr = [0; 20];
    arr[0] = 1;
    let mut i = 1;
    while i < arr.len() {
        arr[i] = arr[i - 1] * i as u64;
        i += 1;
    }
    arr
};
