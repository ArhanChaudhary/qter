use crate::architectures::{Algorithm, PermutationGroup};
use crate::{Int, U, WithSpan};
use std::sync::Arc;

/// The facelets needed for `solved-goto`
#[derive(Debug, Clone)]
pub enum Facelets {
    Theoretical,
    Puzzle { facelets: Vec<usize> },
}

/// The generator of a register along with the facelets needed to decode it
#[derive(Debug, Clone)]
pub enum RegisterGenerator {
    Theoretical,
    Puzzle {
        generator: Box<Algorithm>,
        solved_goto_facelets: Vec<usize>,
    },
}

/// A qter instruction
#[derive(Debug)]
pub enum Instruction {
    Goto {
        instruction_idx: usize,
    },
    SolvedGoto {
        instruction_idx: usize,
        puzzle_idx: usize,
        facelets: Facelets,
    },
    Input {
        message: String,
        puzzle_idx: usize,
        register: RegisterGenerator,
    },
    Halt {
        message: String,
        maybe_puzzle_idx_and_register: Option<(usize, RegisterGenerator)>,
    },
    Print {
        message: String,
        maybe_puzzle_idx_and_register: Option<(usize, RegisterGenerator)>,
    },
    /// Add to a theoretical register; has no representation in .Q
    AddTheoretical {
        puzzle_idx: usize,
        amount: Int<U>,
    },
    Algorithm {
        puzzle_idx: usize,
        algorithm: Algorithm,
    },
}

/// A qter program
#[derive(Debug)]
pub struct Program {
    /// A list of theoretical registers along with their orders
    pub theoretical: Vec<WithSpan<Int<U>>>,
    /// A list of puzzles to be used for registers
    pub puzzles: Vec<WithSpan<Arc<PermutationGroup>>>,
    /// The program itself
    pub instructions: Vec<WithSpan<Instruction>>,
}
