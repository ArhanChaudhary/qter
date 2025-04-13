use std::sync::Arc;

use interpreter::PuzzleState;
use itertools::Itertools;
use qter_core::architectures::{Algorithm, Permutation, PermutationGroup};

pub struct RobotPermutation;

impl PuzzleState for RobotPermutation {
    fn compose_into(&mut self, alg: &Algorithm) {
        println!("moves {}", alg.move_seq_iter().format(" "));
    }

    fn puzzle_state(&self) -> &Permutation {
        // get stdin
        let input = std::io::stdin();
        let mut buffer = String::new();
        input.read_line(&mut buffer).unwrap();
        let input = buffer.trim();
        // parse rob-twophase and return perm
        todo!()
    }

    fn identity(perm_group: Arc<PermutationGroup>) -> Self {
        println!("solve");
        RobotPermutation
    }
}
