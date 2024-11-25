use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::OnceLock,
};

pub mod architectures;
pub mod discrete_math;
mod puzzle_parser;
mod shared_facelet_detection;

use architectures::{Architecture, Permutation};
// Use a huge integers for orders to allow crazy things like examinx
use bnum::types::U512;

#[derive(Clone)]
pub struct Span {
    source: Rc<str>,
    start: usize,
    end: usize,
    line_and_col: OnceLock<(usize, usize)>,
}

impl Span {
    pub fn new(source: Rc<str>, start: usize, end: usize) -> Span {
        assert!(start <= end);
        assert!(start < source.len());
        assert!(end < source.len());

        Span {
            source,
            start,
            end,
            line_and_col: OnceLock::new(),
        }
    }

    pub fn slice(&self) -> &str {
        &self.source[self.start..self.end]
    }

    pub fn line_and_col(&self) -> (usize, usize) {
        *self.line_and_col.get_or_init(|| {
            let mut current_line = 1;
            let mut current_col = 1;

            for c in self.source.chars().take(self.start) {
                if c == '\n' {
                    current_line += 1;
                    current_col = 1;
                } else {
                    current_col += 1;
                }
            }

            (current_line, current_col)
        })
    }

    pub fn line(&self) -> usize {
        self.line_and_col().0
    }

    pub fn col(&self) -> usize {
        self.line_and_col().1
    }
}

/// A value with information about where in the source code the value came from.
///
/// Currently only contains line number information.
pub struct WithSpan<T> {
    pub value: T,
    span: Span,
}

impl<T> Deref for WithSpan<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for WithSpan<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> WithSpan<T> {
    pub fn new(value: T, span: Span) -> WithSpan<T> {
        WithSpan { value, span }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn line(&self) -> usize {
        self.span().line()
    }
}

#[derive(Clone, Debug)]
pub struct PermuteCube {
    cube_idx: usize,
    permutation: Permutation,
    /// Composing the algorithms for each of the registers must give the same result as applying `permutation`
    pub effect: Vec<(usize, U512)>,
}

impl PermuteCube {
    pub fn new(arch: &Architecture, cube_idx: usize, effect: Vec<(usize, U512)>) -> PermuteCube {
        let mut permutation = arch.group().identity();

        for (register, amt) in &effect {
            let mut perm = arch.registers()[*register].permutation.to_owned();

            perm.exponentiate(*amt);

            permutation.compose(&perm);
        }

        PermuteCube {
            permutation,
            effect,
            cube_idx,
        }
    }

    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    pub fn cube_idx(&self) -> usize {
        self.cube_idx
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum RegisterReference {
    Theoretical { idx: usize },
    Puzzle { idx: usize, which_register: usize },
}

pub enum Instruction {
    Goto {
        instruction_idx: usize,
    },
    SolvedGoto {
        instruction_idx: usize,
        register: RegisterReference,
    },
    Input {
        message: String,
        register: RegisterReference,
    },
    Halt {
        message: String,
        register: RegisterReference,
    },
    Print {
        message: String,
        register: RegisterReference,
    },
    AddTheoretical {
        register: usize,
        amount: U512,
    },
    PermuteCube(PermuteCube),
}

/// Represents a qter program
pub struct Program {
    pub theoretical: Vec<WithSpan<U512>>,
    pub puzzles: Vec<WithSpan<Rc<Architecture>>>,
    pub instructions: Vec<WithSpan<Instruction>>,
}
