use std::{
    cell::OnceCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::OnceLock,
};

pub mod architectures;
pub mod discrete_math;
mod puzzle_parser;
mod shared_facelet_detection;

use architectures::{Architecture, Permutation, PermutationGroup};
// Use a huge integers for orders to allow crazy things like examinx
use bnum::types::U512;
use discrete_math::length_of_substring_that_this_string_is_n_repeated_copies_of;
use internment::ArcIntern;

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

/// A value with information about where in the source code the value came from.
///
/// Currently only contains line number information.
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

#[derive(Clone, Debug)]
pub struct PermuteCube {
    cube_idx: usize,
    group: Rc<PermutationGroup>,
    permutation: Permutation,
    /// Composing the algorithms for each of the registers must give the same result as applying `permutation`
    pub effect: Vec<ArcIntern<String>>,
    chromatic_orders: OnceCell<Vec<U512>>,
}

impl PermuteCube {
    pub fn new_from_effect(
        arch: &Architecture,
        cube_idx: usize,
        effect: Vec<(usize, U512)>,
    ) -> PermuteCube {
        let mut permutation = arch.group().identity();

        let mut generators = Vec::new();

        // TODO: Refactor once the puzzle definition includes optimized generators for various combinations of effects
        for (register, amt) in &effect {
            let reg = &arch.registers()[*register];
            let mut perm = reg.permutation.to_owned();

            perm.exponentiate(*amt);

            permutation.compose(&perm);

            let mut i = U512::ZERO;
            while i < *amt {
                generators.extend_from_slice(reg.generator_sequence());
                i += U512::ONE;
            }
        }

        PermuteCube {
            permutation,
            effect: generators,
            cube_idx,
            group: arch.group_rc(),
            chromatic_orders: OnceCell::new(),
        }
    }

    pub fn new_from_generators<'a>(
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
            effect: generators,
            chromatic_orders: OnceCell::new(),
        })
    }

    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    pub fn cube_idx(&self) -> usize {
        self.cube_idx
    }

    pub fn chromatic_orders_by_facelets(&self) -> &[U512] {
        self.chromatic_orders.get_or_init(|| {
            let mut out = vec![U512::ONE; self.group.facelet_count()];

            self.permutation().cycles().iter().for_each(|cycle| {
                let chromatic_order = length_of_substring_that_this_string_is_n_repeated_copies_of(
                    cycle.iter().map(|v| &**self.group.facelet_colors()[*v]),
                );

                for facelet in cycle {
                    out[*facelet] = chromatic_order;
                }
            });

            out
        })
    }
}

#[derive(Debug, Clone)]
pub enum Facelets {
    Theoretical,
    Puzzle { facelets: Vec<usize> },
}

#[derive(Debug, Clone)]
pub enum RegisterGenerator {
    Theoretical,
    Puzzle {
        generator: PermuteCube,
        facelets: Vec<usize>,
    },
}

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
    AddTheoretical {
        register_idx: usize,
        amount: U512,
    },
    PermuteCube(PermuteCube),
}

/// Represents a qter program
pub struct Program {
    pub theoretical: Vec<WithSpan<U512>>,
    pub puzzles: Vec<WithSpan<Rc<PermutationGroup>>>,
    pub instructions: Vec<WithSpan<Instruction>>,
}
