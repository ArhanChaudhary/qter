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
use internment::ArcIntern;

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

/// Represents a collection of registers, each represented in a particular group structuree
///
/// Corresponds to a line of the `.registers` declaration
pub enum RegisterRepresentation {
    Theoretical { name: String, order: U512 },
    Puzzle(Rc<Architecture>),
}

pub enum Instruction {
    Goto {
        instruction_idx: usize,
    },
    SolvedGoto {
        instruction_idx: usize,
        register: String,
    },
    Input {
        message: String,
        register: String,
    },
    Halt {
        message: String,
        register: String,
    },
    Print {
        message: String,
        register: String,
    },
    AddTheoretical {
        register: String,
        amount: U512,
    },
    PermuteCube {
        permutation: Permutation,
        effect: Vec<(ArcIntern<String>, U512)>,
    },
}

/// Represents a qter program
pub struct Program {
    pub groups: Vec<WithSpan<RegisterRepresentation>>,
    pub instructions: Vec<WithSpan<Instruction>>,
}
