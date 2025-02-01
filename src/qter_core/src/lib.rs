use std::{
    cell::OnceCell,
    ops::{Deref, DerefMut},
    sync::{Arc, OnceLock},
};

pub mod architectures;
mod math;
mod puzzle_parser;
mod shared_facelet_detection;
pub mod table_encoding;

use discrete_math::length_of_substring_that_this_string_is_n_repeated_copies_of;
pub use math::*;

use architectures::{Architecture, Permutation, PermutationGroup};
// Use a huge integers for orders to allow crazy things like examinx
use internment::ArcIntern;
use pest::{Position, RuleType};

pub fn mk_error<Rule: RuleType>(
    message: impl Into<String>,
    loc: impl AsPestLoc,
) -> Box<pest::error::Error<Rule>> {
    let err = pest::error::ErrorVariant::CustomError {
        message: message.into(),
    };

    return Box::new(match loc.as_pest_loc() {
        SpanOrPos::Span(span) => pest::error::Error::new_from_span(err, span),
        SpanOrPos::Pos(pos) => pest::error::Error::new_from_pos(err, pos),
    });
}

pub enum SpanOrPos<'a> {
    Span(pest::Span<'a>),
    Pos(pest::Position<'a>),
}

pub trait AsPestLoc {
    fn as_pest_loc(&self) -> SpanOrPos<'_>;
}

impl<'a> AsPestLoc for pest::Span<'a> {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        SpanOrPos::Span(self.to_owned())
    }
}

impl AsPestLoc for Span {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        SpanOrPos::Span(self.pest())
    }
}

impl<'a> AsPestLoc for Position<'a> {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        SpanOrPos::Pos(self.to_owned())
    }
}

impl<T: AsPestLoc> AsPestLoc for &T {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        (*self).as_pest_loc()
    }
}

/// A slice of the original source code; to be attached to pieces of data for error reporting
#[derive(Clone)]
pub struct Span {
    source: ArcIntern<str>,
    start: usize,
    end: usize,
    line_and_col: OnceLock<(usize, usize)>,
}

impl Span {
    pub fn from_span(span: pest::Span) -> Span {
        Span::new(ArcIntern::from(span.get_input()), span.start(), span.end())
    }

    pub fn new(source: ArcIntern<str>, start: usize, end: usize) -> Span {
        assert!(start <= end);
        assert!(start < source.len());
        assert!(end <= source.len());

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

    pub fn after(mut self) -> Span {
        self.start = self.end;
        self
    }

    pub fn source(&self) -> ArcIntern<str> {
        ArcIntern::clone(&self.source)
    }

    pub fn merge(self, other: &Span) -> Span {
        assert_eq!(self.source, other.source);

        Span {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line_and_col: OnceLock::new(),
        }
    }

    fn pest(&self) -> pest::Span<'_> {
        pest::Span::new(&self.source, self.start, self.end).unwrap()
    }
}

impl core::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slice())
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(value: pest::Span) -> Self {
        Span::from_span(value)
    }
}

/// A value attached to a `Span`
#[derive(Clone)]
pub struct WithSpan<T> {
    pub value: T,
    span: Span,
}

impl<T: core::fmt::Debug> core::fmt::Debug for WithSpan<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Debug::fmt(&self.value, f)
    }
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

    pub fn into_inner(self) -> T {
        self.value
    }

    pub fn map<V>(self, f: impl FnOnce(T) -> V) -> WithSpan<V> {
        WithSpan {
            value: f(self.value),
            span: self.span,
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn line(&self) -> usize {
        self.span().line()
    }
}

impl<T: PartialEq> PartialEq for WithSpan<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for WithSpan<T> {}

impl<T: core::hash::Hash> core::hash::Hash for WithSpan<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

/// Represents a sequence of moves to apply to a puzzle in the `Program`
#[derive(Clone)]
pub struct PermutePuzzle {
    group: Arc<PermutationGroup>,
    permutation: Permutation,
    generators: Vec<ArcIntern<str>>,
    chromatic_orders: OnceCell<Vec<Int<U>>>,
}

impl PermutePuzzle {
    /// Create a `PermutePuzzle` from what values it should add to which registers.
    ///
    /// `effect` is a list of tuples of register indices and how much to add to add to them.
    pub fn new_from_effect(arch: &Architecture, effect: Vec<(usize, Int<U>)>) -> PermutePuzzle {
        let mut permutation = arch.group().identity();

        let mut generators = Vec::new();

        // TODO: Refactor once the puzzle definition includes optimized generators for various combinations of effects
        for (register, amt) in &effect {
            let reg = &arch.registers()[*register];
            let mut perm = reg.permutation.to_owned();

            perm.exponentiate((*amt).into());

            permutation.compose(&perm);

            let mut i = Int::<U>::zero();
            while i < *amt {
                generators.extend_from_slice(reg.generator_sequence());
                i += Int::<U>::one();
            }
        }

        PermutePuzzle {
            permutation,
            generators,
            group: arch.group_arc(),
            chromatic_orders: OnceCell::new(),
        }
    }

    /// Create a `PermutePuzzle` instance from a list of generators
    pub fn new_from_generators(
        group: Arc<PermutationGroup>,
        generators: Vec<ArcIntern<str>>,
    ) -> Result<PermutePuzzle, ArcIntern<str>> {
        let mut permutation = group.identity();

        group.compose_generators_into(&mut permutation, generators.iter())?;

        Ok(PermutePuzzle {
            group,
            permutation,
            generators,
            chromatic_orders: OnceCell::new(),
        })
    }

    /// Get the underlying permutation of the `PermutePuzzle` instance
    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    /// Returns a list of generators that when composed, give the same result as applying `.permutation()`
    pub fn generators(&self) -> &[ArcIntern<str>] {
        &self.generators
    }

    /// Calculate the order of every cycle of facelets created by seeing this `PermutePuzzle` instance as a register generator.
    ///
    /// Returns a list of chromatic orders where the index is the facelet.
    pub fn chromatic_orders_by_facelets(&self) -> &[Int<U>] {
        self.chromatic_orders.get_or_init(|| {
            let mut out = vec![Int::one(); self.group.facelet_count()];

            self.permutation().cycles().iter().for_each(|cycle| {
                let chromatic_order = length_of_substring_that_this_string_is_n_repeated_copies_of(
                    cycle.iter().map(|v| &*self.group.facelet_colors()[*v]),
                );

                for facelet in cycle {
                    out[*facelet] = Int::from(chromatic_order);
                }
            });

            out
        })
    }
}

impl core::fmt::Debug for PermutePuzzle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermutePuzzle")
            .field("permutation", &self.permutation)
            // .field(
            //     "generators",
            //     &self
            //         .generators
            //         .iter()
            //         .map(|v| &**v)
            //         .intersperse(" ")
            //         .collect::<String>(),
            // )
            .finish_non_exhaustive()
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
        generator: PermutePuzzle,
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
        register_idx: Option<usize>,
        register: Option<RegisterGenerator>,
    },
    Print {
        message: String,
        register_idx: Option<usize>,
        register: Option<RegisterGenerator>,
    },
    /// Add to a theoretical register; has no representation in .Q
    AddTheoretical {
        register_idx: usize,
        amount: Int<U>,
    },
    PermutePuzzle {
        puzzle_idx: usize,
        permute_puzzle: PermutePuzzle,
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
