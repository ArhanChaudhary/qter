use std::{
    cell::OnceCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::OnceLock,
};

pub mod architectures;
pub mod discrete_math;
mod numbers;
mod puzzle_parser;
mod shared_facelet_detection;

pub use numbers::*;

use architectures::{Architecture, Permutation, PermutationGroup};
// Use a huge integers for orders to allow crazy things like examinx
use discrete_math::length_of_substring_that_this_string_is_n_repeated_copies_of;
use internment::ArcIntern;

/// A slice of the original source code; to be attached to pieces of data for error reporting
#[derive(Debug, Clone)]
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

/// A value attached to a `Span`
#[derive(Debug)]
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

/// Represents a sequence of moves to apply to a cube in the `Program`
#[derive(Clone, Debug)]
pub struct PermuteCube {
    cube_idx: usize,
    group: Rc<PermutationGroup>,
    permutation: Permutation,
    generators: Vec<ArcIntern<String>>,
    chromatic_orders: OnceCell<Vec<Int<U>>>,
}

impl PermuteCube {
    /// Create a `PermuteCube` from what values it should add to which registers.
    ///
    /// `effect` is a list of tuples of register indices and how much to add to add to them.
    pub fn new_from_effect(
        arch: &Architecture,
        cube_idx: usize,
        effect: Vec<(usize, Int<U>)>,
    ) -> PermuteCube {
        let mut permutation = arch.group().identity();

        let mut generators = Vec::new();

        // TODO: Refactor once the puzzle definition includes optimized generators for various combinations of effects
        for (register, amt) in &effect {
            let reg = &arch.registers()[*register];
            let mut perm = reg.permutation.to_owned();

            perm.exponentiate(*amt);

            permutation.compose(&perm);

            let mut i = Int::<U>::zero();
            while i < *amt {
                generators.extend_from_slice(reg.generator_sequence());
                i += Int::<U>::one();
            }
        }

        PermuteCube {
            permutation,
            generators,
            cube_idx,
            group: arch.group_rc(),
            chromatic_orders: OnceCell::new(),
        }
    }

    /// Create a `PermuteCube` instance from a list of generators
    pub fn new_from_generators(
        group: Rc<PermutationGroup>,
        cube_idx: usize,
        generators: Vec<ArcIntern<String>>,
    ) -> Result<PermuteCube, ArcIntern<String>> {
        let mut permutation = group.identity();

        group.compose_generators_into(&mut permutation, generators.iter())?;

        Ok(PermuteCube {
            cube_idx,
            group,
            permutation,
            generators,
            chromatic_orders: OnceCell::new(),
        })
    }

    /// Get the underlying permutation of the `PermuteCube` instance
    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    /// Get the index of the cube that this is intented to be applied to
    pub fn cube_idx(&self) -> usize {
        self.cube_idx
    }

    /// Returns a list of generators that when composed, give the same result as applying `.permutation()`
    pub fn generators(&self) -> &[ArcIntern<String>] {
        &self.generators
    }

    /// Calculate the order of every cycle of facelets created by seeing this `PermuteCube` instance as a register generator.
    ///
    /// Returns a list of chromatic orders where the index is the facelet.
    pub fn chromatic_orders_by_facelets(&self) -> &[Int<U>] {
        self.chromatic_orders.get_or_init(|| {
            let mut out = vec![Int::one(); self.group.facelet_count()];

            self.permutation().cycles().iter().for_each(|cycle| {
                let chromatic_order = length_of_substring_that_this_string_is_n_repeated_copies_of(
                    cycle.iter().map(|v| &**self.group.facelet_colors()[*v]),
                );

                for facelet in cycle {
                    out[*facelet] = Int::from(chromatic_order);
                }
            });

            out
        })
    }
}

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
        generator: PermuteCube,
        facelets: Vec<usize>,
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
        register_idx: usize,
        facelets: Facelets,
    },
    Input {
        message: String,
        register_idx: usize,
        register: RegisterGenerator,
    },
    Halt {
        message: String,
        register_idx: usize,
        register: RegisterGenerator,
    },
    Print {
        message: String,
        register_idx: usize,
        register: RegisterGenerator,
    },
    /// Add to a theoretical register; has no representation in .Q
    AddTheoretical {
        register_idx: usize,
        amount: Int<U>,
    },
    PermuteCube(PermuteCube),
}

/// A qter program
pub struct Program {
    /// A list of theoretical registers along with their orders
    pub theoretical: Vec<WithSpan<Int<U>>>,
    /// A list of puzzles to be used for registers
    pub puzzles: Vec<WithSpan<Rc<PermutationGroup>>>,
    /// The program itself
    pub instructions: Vec<WithSpan<Instruction>>,
}
