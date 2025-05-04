pub mod canonical_fsm;
pub mod pruning;
pub mod puzzle;
pub mod puzzle_state_history;
pub mod solver;

const FACT_UNTIL_20: [u64; 21] = {
    let mut arr = [0; 21];
    arr[0] = 1;
    let mut i = 1;
    while i <= 20 {
        arr[i] = arr[i - 1] * i as u64;
        i += 1;
    }
    arr
};
