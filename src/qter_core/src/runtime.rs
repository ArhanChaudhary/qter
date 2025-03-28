use crate::architectures::{Architecture, Permutation, PermutationGroup};
use crate::discrete_math::length_of_substring_that_this_string_is_n_repeated_copies_of;
use crate::{I, Int, U, WithSpan};
use internment::ArcIntern;
use itertools::Itertools;
use std::{cell::OnceCell, sync::Arc};

/// Represents a sequence of moves to apply to a puzzle in the `Program`
#[derive(Clone)]
pub struct Algorithm {
    group: Arc<PermutationGroup>,
    permutation: Permutation,
    generators: Vec<ArcIntern<str>>,
    chromatic_orders: OnceCell<Vec<Int<U>>>,
    repeat: Int<I>,
}

impl Algorithm {
    /// Create an `Algorithm` from what values it should add to which registers.
    ///
    /// `effect` is a list of tuples of register indices and how much to add to add to them.
    pub fn new_from_effect(arch: &Architecture, effect: Vec<(usize, Int<U>)>) -> Algorithm {
        let mut generators = Vec::new();

        let mut expanded_effect = vec![Int::<U>::zero(); arch.registers().len()];

        for (register, amt) in effect {
            expanded_effect[register] = amt;
        }

        let table = arch.get_table();
        let orders = table.orders();

        while expanded_effect.iter().any(|v| !v.is_zero()) {
            let (true_effect, alg) = table.closest_alg(&expanded_effect);

            expanded_effect
                .iter_mut()
                .zip(true_effect.iter().copied())
                .zip(orders.iter().copied())
                .for_each(|((expanded_effect, true_effect), order)| {
                    *expanded_effect = if *expanded_effect < true_effect {
                        *expanded_effect + order - true_effect
                    } else {
                        *expanded_effect - true_effect
                    }
                });

            generators.extend_from_slice(alg);
        }

        Self::new_from_generators(arch.group_arc(), generators).unwrap()
    }

    /// Create an `Algorithm` instance from a list of generators
    pub fn new_from_generators(
        group: Arc<PermutationGroup>,
        generators: Vec<ArcIntern<str>>,
    ) -> Result<Algorithm, ArcIntern<str>> {
        let mut permutation = group.identity();

        group.compose_generators_into(&mut permutation, generators.iter())?;

        Ok(Algorithm {
            group,
            permutation,
            generators,
            chromatic_orders: OnceCell::new(),
            repeat: Int::<I>::one(),
        })
    }

    /// Get the underlying permutation of the `Algorithm` instance
    pub fn permutation(&self) -> &Permutation {
        &self.permutation
    }

    /// Find the result of applying the algorithm to the identity `exponent` times.
    ///
    /// This calculates the value in O(1) time with respect to `exponent`.
    pub fn exponentiate(&mut self, exponent: Int<I>) {
        self.repeat = exponent;
        self.permutation.exponentiate(exponent);
    }

    /// Returns a list of generators that when composed, give the same result as applying `.permutation()`
    pub fn generators(&self) -> &[ArcIntern<str>] {
        &self.generators
    }

    /// Return the permutation group that this alg operates on
    pub fn group(&self) -> &PermutationGroup {
        &self.group
    }

    pub fn repeat(&self) -> Int<I> {
        self.repeat
    }

    /// Return the permutation group that this alg operates on in an Arc
    pub fn group_arc(&self) -> Arc<PermutationGroup> {
        Arc::clone(&self.group)
    }

    /// Calculate the order of every cycle of facelets created by seeing this `Algorithm` instance as a register generator.
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

impl core::fmt::Debug for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Algorithm")
            .field("permutation", &self.permutation)
            .field(
                "generators",
                &self.generators.iter().map(|v| &**v).join(" "),
            )
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
#[allow(clippy::large_enum_variant)]
pub enum RegisterGenerator {
    Theoretical,
    Puzzle {
        generator: Algorithm,
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
