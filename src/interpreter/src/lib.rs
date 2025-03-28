use std::{collections::VecDeque, mem, sync::Arc};

use qter_core::{
    Algorithm, Facelets, I, Instruction, Int, Program, RegisterGenerator, U,
    architectures::{Permutation, PermutationGroup},
    discrete_math::{decode, lcm},
};

/// Represents an instance of a `PermutationGroup`, in other words this simulates the puzzle
pub struct Puzzle {
    group: Arc<PermutationGroup>,
    state: Permutation,
}

impl Puzzle {
    /// Initialize the `Puzzle` in the solved state
    pub fn initialize(group: Arc<PermutationGroup>) -> Puzzle {
        Puzzle {
            state: group.identity(),
            group,
        }
    }

    /// Check whether the given facelets are solved
    pub fn facelets_solved(&self, facelets: &[usize]) -> bool {
        for facelet in facelets {
            let maps_to = self.state().mapping()[*facelet];
            if self.group.facelet_colors()[maps_to] != self.group.facelet_colors()[*facelet] {
                return false;
            }
        }

        true
    }

    /// Decode the permutation using the register generator and the given facelets.
    ///
    /// In general, an arbitrary scramble cannot be decoded. If this is the case, the function will return `None`.
    pub fn decode(&self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        decode(self.state(), facelets, generator)
    }

    /// Get the underlying `PermutationGroup` of the puzzle
    pub fn group(&self) -> &PermutationGroup {
        &self.group
    }

    /// Get the current state of the puzzle
    pub fn state(&self) -> &Permutation {
        &self.state
    }
}

/// If the interpreter is paused, this represents the reason why.
pub enum PausedState {
    Halt {
        register_idx: Option<usize>,
        register: Option<RegisterGenerator>,
    },
    Input {
        max_input: Int<I>,
        register_idx: usize,
        register: RegisterGenerator,
    },
    Panicked,
}

/// Whether the interpreter can be stepped forward or is paused for some reason
#[allow(clippy::large_enum_variant)]
pub enum ExecutionState {
    Running,
    Paused(PausedState),
}

/// An instance of a theoretical register. Analagous to the `Puzzle` structure.
struct TheoreticalState {
    state: Int<U>,
    order: Int<U>,
}

impl TheoreticalState {
    fn add_to_i(&mut self, amt: Int<I>) {
        self.add_to(amt % self.order);
    }

    fn add_to(&mut self, amt: Int<U>) {
        assert!(amt < self.order);

        self.state += amt % self.order;

        if self.state >= self.order {
            self.state -= self.order;
        }
    }
}

/// A collection of the states of every puzzle and theoretical register
///
/// Factored out for borrow checker reasons
struct PuzzleStates {
    theoretical_states: Vec<TheoreticalState>,
    puzzle_states: Vec<Puzzle>,
}

/// An interpreter for a qter program
pub struct Interpreter {
    puzzle_states: PuzzleStates,
    program_counter: usize,
    program: Program,
    messages: VecDeque<String>,
    execution_state: ExecutionState,
}

/// The action performed by the instruction that was just executed
pub enum ActionPerformed<'s> {
    None,
    Paused,
    Goto {
        location: usize,
    },
    FailedSolvedGoto {
        puzzle_idx: usize,
        facelets: &'s Facelets,
    },
    SucceededSolvedGoto {
        puzzle_idx: usize,
        facelets: &'s Facelets,
        location: usize,
    },
    AddedToTheoretical {
        register_idx: usize,
        amt: Int<U>,
    },
    ExecutedAlgorithm {
        puzzle_idx: usize,
        algorithm: &'s Algorithm,
    },
    Panicked,
}

impl PuzzleStates {
    /// Check whether a register is solved
    fn is_register_solved(&self, register_idx: usize, which_reg: &Facelets) -> bool {
        match which_reg {
            Facelets::Theoretical => self.theoretical_states[register_idx].state.is_zero(),
            Facelets::Puzzle { facelets } => {
                let puzzle = &self.puzzle_states[register_idx];
                puzzle.facelets_solved(facelets)
            }
        }
    }

    /// Decode a register
    fn decode_register(
        &self,
        register_idx: usize,
        which_reg: &RegisterGenerator,
    ) -> Option<Int<U>> {
        match which_reg {
            RegisterGenerator::Theoretical => Some(self.theoretical_states[register_idx].state),
            RegisterGenerator::Puzzle {
                generator,
                facelets,
            } => {
                let puzzle = &self.puzzle_states[register_idx];
                puzzle.decode(facelets, generator)
            }
        }
    }

    fn register_order(&self, which_reg: &RegisterGenerator, register_idx: usize) -> Int<U> {
        match which_reg {
            RegisterGenerator::Theoretical => self.theoretical_states[register_idx].order,
            RegisterGenerator::Puzzle {
                generator,
                facelets,
            } => facelets
                .iter()
                .map(|facelet| generator.chromatic_orders_by_facelets()[*facelet])
                .fold(Int::<U>::one(), lcm),
        }
    }

    /// Compose a permutation into a puzzle state
    fn compose_into(&mut self, puzzle_idx: usize, permutation: &Permutation) {
        self.puzzle_states[puzzle_idx].state.compose(permutation);
    }
}

impl Interpreter {
    /// Create a new interpreter from a program and initial states for registers
    ///
    /// If an initial state isn't specified, it defaults to zero.
    pub fn new(program: Program) -> Self {
        let theoretical_states = program
            .theoretical
            .iter()
            .map(|order| TheoreticalState {
                state: Int::zero(),
                order: **order,
            })
            .collect();

        let puzzle_states = program
            .puzzles
            .iter()
            .map(|group| Puzzle::initialize(Arc::clone(group)))
            .collect();

        Interpreter {
            puzzle_states: PuzzleStates {
                theoretical_states,
                puzzle_states,
            },
            program,
            program_counter: 0,
            messages: VecDeque::new(),
            execution_state: ExecutionState::Running,
        }
    }

    /// Return the instruction index to be executed next
    pub fn program_counter(&self) -> usize {
        self.program_counter
    }

    /// The program currently being executed
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Get the current execution state of the interpreter
    pub fn execution_state(&self) -> &ExecutionState {
        &self.execution_state
    }

    /// Get the message queue of the interpreter
    pub fn messages(&mut self) -> &mut VecDeque<String> {
        &mut self.messages
    }

    /// Execute one instruction
    pub fn step(&mut self) -> ActionPerformed<'_> {
        macro_rules! interpreter_panic {
            ($self:ident, $message:expr) => {{
                $self.execution_state = ExecutionState::Paused(PausedState::Panicked);
                $self.messages.push_back(format!("Panicked: {{$message}}"));
                ActionPerformed::Panicked
            }};
        }

        if let ExecutionState::Paused(_) = self.execution_state() {
            return ActionPerformed::Paused;
        }
        let instruction = match self.program.instructions.get(self.program_counter) {
            Some(v) => v,
            None => {
                return interpreter_panic!(
                    self,
                    "Execution fell through the end of the program without reaching a halt instruction!"
                );
            }
        };

        match &**instruction {
            Instruction::Goto { instruction_idx } => {
                self.program_counter = *instruction_idx;
                self.execution_state = ExecutionState::Running;

                ActionPerformed::Goto {
                    location: *instruction_idx,
                }
            }
            Instruction::SolvedGoto {
                instruction_idx,
                facelets,
                register_idx,
            } => {
                self.execution_state = ExecutionState::Running;
                if self
                    .puzzle_states
                    .is_register_solved(*register_idx, facelets)
                {
                    self.program_counter = *instruction_idx;

                    ActionPerformed::SucceededSolvedGoto {
                        facelets,
                        location: *instruction_idx,
                        puzzle_idx: *register_idx,
                    }
                } else {
                    self.program_counter += 1;

                    ActionPerformed::FailedSolvedGoto {
                        facelets,
                        puzzle_idx: *register_idx,
                    }
                }
            }
            Instruction::Input {
                message,
                register,
                register_idx,
            } => {
                let max_input =
                    self.puzzle_states.register_order(register, *register_idx) - Int::<I>::one();
                self.execution_state = ExecutionState::Paused(PausedState::Input {
                    max_input,
                    register: register.to_owned(),
                    register_idx: *register_idx,
                });
                self.messages
                    .push_back(format!("{message} (max input {max_input})"));

                ActionPerformed::Paused
            }
            Instruction::Halt {
                message,
                register,
                register_idx,
            } => {
                self.execution_state = ExecutionState::Paused(PausedState::Halt {
                    register: register.to_owned(),
                    register_idx: *register_idx,
                });
                let full_message = if register.is_none() {
                    message.to_owned()
                } else {
                    match self
                        .puzzle_states
                        .decode_register(register_idx.unwrap(), register.as_ref().unwrap())
                    {
                        Some(v) => format!("{message} {v}"),
                        None => {
                            return interpreter_panic!(
                                self,
                                "The register specified is not decodable!"
                            );
                        }
                    }
                };
                self.messages.push_back(full_message);

                ActionPerformed::Paused
            }
            Instruction::Print {
                message,
                register,
                register_idx,
            } => {
                self.execution_state = ExecutionState::Running;
                let full_message = if register.is_none() {
                    message.to_owned()
                } else {
                    match self
                        .puzzle_states
                        .decode_register(register_idx.unwrap(), register.as_ref().unwrap())
                    {
                        Some(v) => format!("{message} {v}",),
                        None => {
                            return interpreter_panic!(
                                self,
                                "The register specified is not decodable!"
                            );
                        }
                    }
                };
                self.messages.push_back(full_message);
                self.program_counter += 1;

                ActionPerformed::None
            }
            Instruction::AddTheoretical {
                register_idx,
                amount,
            } => {
                self.execution_state = ExecutionState::Running;

                self.puzzle_states.theoretical_states[*register_idx].add_to(*amount);

                self.program_counter += 1;

                ActionPerformed::AddedToTheoretical {
                    register_idx: *register_idx,
                    amt: *amount,
                }
            }
            Instruction::Algorithm {
                algorithm: permute_puzzle,
                puzzle_idx,
            } => {
                self.execution_state = ExecutionState::Running;
                self.puzzle_states
                    .compose_into(*puzzle_idx, permute_puzzle.permutation());

                self.program_counter += 1;

                ActionPerformed::ExecutedAlgorithm {
                    puzzle_idx: *puzzle_idx,
                    algorithm: permute_puzzle,
                }
            }
        }
    }

    /// Execute instructions until an input or halt instruction is reached
    ///
    /// Returns details of the paused state reached
    pub fn step_until_halt(&mut self) -> &PausedState {
        loop {
            if let ActionPerformed::Paused | ActionPerformed::Panicked = self.step() {
                break;
            }
        }
        match self.execution_state() {
            ExecutionState::Paused(v) => v,
            ExecutionState::Running => panic!("Cannot be halted while running"),
        }
    }

    /// Give an input to the interpreter, returning the puzzle index and the algorithm performed `value` times if applicable
    ///
    /// Panics if the interpreter is not executing an `input` instruction
    pub fn give_input(&mut self, value: Int<I>) -> Result<(usize, Option<Algorithm>), String> {
        let ExecutionState::Paused(PausedState::Input {
            max_input,
            register_idx: _,
            register: _,
        }) = &self.execution_state
        else {
            panic!("The interpreter isn't in an input state");
        };

        if value > *max_input {
            return Err(format!(
                "Your input must not be greater than {}.",
                max_input
            ));
        }
        if value < -*max_input {
            return Err(format!("Your input must not be less than {}.", -*max_input));
        }

        // The code is weird to appease the borrow checker

        let ExecutionState::Paused(PausedState::Input {
            max_input: _,
            register_idx,
            register,
        }) = mem::replace(&mut self.execution_state, ExecutionState::Running)
        else {
            unreachable!("Checked before")
        };

        let alg = match register {
            RegisterGenerator::Theoretical => {
                self.puzzle_states.theoretical_states[register_idx].add_to_i(value);

                None
            }
            RegisterGenerator::Puzzle {
                generator,
                facelets: _,
            } => {
                let puzzle = &mut self.puzzle_states.puzzle_states[register_idx];
                let mut perm = generator.permutation().to_owned();

                perm.exponentiate(value);

                puzzle.state.compose(&perm);

                Some(generator)
            }
        };

        self.execution_state = ExecutionState::Running;
        self.program_counter += 1;

        Ok((register_idx, alg))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use compiler::compile;
    use internment::ArcIntern;
    use qter_core::{Algorithm, Int, RegisterGenerator, U, architectures::PuzzleDefinition};

    use crate::{Interpreter, PausedState, Puzzle};

    #[test]
    fn facelets_solved() {
        let group = Arc::new(
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap(),
        );

        let mut cube = Puzzle::initialize(Arc::clone(&group.group));

        // Remember that the decoder will subtract the smallest facelet found in the definition to make it zero based
        assert!(cube.facelets_solved(&[0, 8, 16, 24]));

        group
            .group
            .compose_generators_into(&mut cube.state, [ArcIntern::from("U")].iter())
            .unwrap();

        assert!(cube.facelets_solved(&[0, 12, 15, 7, 40]));

        assert!(!cube.facelets_solved(&[1, 12, 15, 7, 24]));
    }

    #[test]
    fn complicated_solved_decode_test() {
        let group = Arc::new(
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap(),
        );

        let arch = group
            .get_preset(&[Int::from(210_u64), Int::from(24_u64)])
            .unwrap();

        let a_facelets = arch.registers()[0].signature_facelets();
        let b_facelets = arch.registers()[1].signature_facelets();

        let a_permutation = Algorithm::new_from_effect(&arch, vec![(0, Int::one())]);
        let b_permutation = Algorithm::new_from_effect(&arch, vec![(1, Int::one())]);

        let mut cube = Puzzle::initialize(Arc::clone(&group.group));

        for i in 1..=23 {
            cube.state.compose(b_permutation.permutation());
            assert_eq!(
                cube.decode(&b_facelets, &b_permutation).unwrap(),
                Int::from(i)
            );
            assert!(!cube.facelets_solved(&b_facelets));
        }

        cube.state.compose(b_permutation.permutation());
        assert!(cube.facelets_solved(&b_facelets));
        assert_eq!(
            cube.decode(&b_facelets, &b_permutation).unwrap(),
            Int::<U>::zero()
        );

        for i in 0..24 {
            println!("{i}");
            for j in 0..210 {
                assert_eq!(
                    cube.decode(&b_facelets, &b_permutation).unwrap(),
                    Int::from(i)
                );
                assert_eq!(
                    cube.decode(&a_facelets, &a_permutation).unwrap(),
                    Int::from(j)
                );

                cube.state.compose(a_permutation.permutation());
            }

            cube.state.compose(b_permutation.permutation());
        }
    }

    #[test]
    fn modulus() {
        let code = "
            .registers {
                B, A ← 3x3 builtin (24, 210)
            }

                input \"Number to modulus:\" A
            loop:
                print \"A is now\" A
                add B 13
            decrement:
                solved-goto B loop
                solved-goto A fix
                add A 209
                add B 23
                goto decrement
            fix:
                solved-goto B finalize
                add A 209
                add B 23
                goto fix
            finalize:
                add A 13
                halt \"The modulus is\" A
        ";

        let program = match compile(code, |_| unreachable!()) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };

        let mut interpreter = Interpreter::new(program);

        assert!(match interpreter.step_until_halt() {
            PausedState::Input {
                max_input,
                register:
                    RegisterGenerator::Puzzle {
                        generator: _,
                        facelets: _,
                    },
                register_idx: 0,
            } => *max_input == Int::from(209),
            _ => false,
        });

        assert!(interpreter.give_input(Int::from(133_u64)).is_ok());

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                register: Some(RegisterGenerator::Puzzle {
                    generator: _,
                    facelets: _,
                }),
                register_idx: Some(0),
            }
        ));

        let expected_output = [
            "Number to modulus: (max input 209)",
            "A is now 133",
            "A is now 120",
            "A is now 107",
            "A is now 94",
            "A is now 81",
            "A is now 68",
            "A is now 55",
            "A is now 42",
            "A is now 29",
            "A is now 16",
            "A is now 3",
            "The modulus is 3",
        ];

        assert_eq!(
            expected_output.len(),
            interpreter.messages().len(),
            "{:?}",
            interpreter.messages()
        );

        for (message, expected) in interpreter.messages.iter().zip(expected_output.iter()) {
            assert_eq!(message, expected);
        }
    }

    #[test]
    fn fib() {
        // TODO: a test directory of qat files?
        let code = "
            .registers {
                D, C, B, A ← 3x3 builtin (9, 10, 18, 30)
            }

                input \"Which Fibonacci number to calculate:\" D
                solved-goto D do_if_1
                goto after_if_1
            do_if_1:
                halt \"The number is 0\"
            after_if_1:
                add B 1
            continue_1:
                add D 8
                solved-goto D do_if_2
                goto after_if_2
            do_if_2:
                halt \"The number is\" B
            after_if_2:
            continue_2:
                solved-goto B break_2
                add B 17
                add A 1
                add C 1
                goto continue_2
            break_2:
                add D 8
                solved-goto D do_if_3
                goto after_if_3
            do_if_3:
                halt \"The number is\" A
            after_if_3:
            continue_3:
                solved-goto A break_3
                add A 29
                add C 1
                add B 1
                goto continue_3
            break_3:
                add D 8
                solved-goto D do_if_4
                goto after_if_4
            do_if_4:
                halt \"The number is\" C
            after_if_4:
            continue_4:
                solved-goto C break_4
                add C 9
                add B 1
                add A 1
                goto continue_4
            break_4:
                goto continue_1
        ";

        let program = match compile(code, |_| unreachable!()) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };

        let mut interpreter = Interpreter::new(program);

        assert!(match interpreter.step_until_halt() {
            PausedState::Input {
                max_input,
                register:
                    RegisterGenerator::Puzzle {
                        generator: _,
                        facelets: _,
                    },
                register_idx: 0,
            } => *max_input == Int::from(8),
            _ => false,
        });

        assert!(interpreter.give_input(Int::from(8_u64)).is_ok());

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                register: Some(RegisterGenerator::Puzzle {
                    generator: _,
                    facelets: _,
                }),
                register_idx: Some(0),
            }
        ));

        let expected_output = [
            "Which Fibonacci number to calculate: (max input 8)",
            "The number is 21",
        ];

        assert_eq!(
            expected_output.len(),
            interpreter.messages().len(),
            "{:?}",
            interpreter.messages()
        );

        for (message, expected) in interpreter.messages.iter().zip(expected_output.iter()) {
            assert_eq!(message, expected);
        }
    }
}
