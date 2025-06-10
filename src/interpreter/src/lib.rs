#![warn(clippy::pedantic)]
#![allow(clippy::too_many_lines)]

mod instructions;

use std::{collections::VecDeque, mem, sync::Arc};

use instructions::do_instr;
use qter_core::{
    ByPuzzleType, Facelets, I, Instruction, Int, Program, RegisterGenerator, SeparatesByPuzzleType,
    U,
    architectures::{Algorithm, Permutation, PermutationGroup},
    discrete_math::decode,
};

/// If the interpreter is paused, this represents the reason why.
#[derive(Debug)]
pub enum PausedState {
    Halt {
        maybe_puzzle_idx_and_register: Option<(usize, ByPuzzleType<'static, RegisterGenerator>)>,
    },
    Input {
        max_input: Int<U>,
        register: ByPuzzleType<'static, RegisterGenerator>,
        puzzle_idx: usize,
    },
    Panicked,
}

/// Whether the interpreter can be stepped forward or is paused for some reason
pub enum ExecutionState {
    Running,
    Paused(PausedState),
}

/// An instance of a theoretical register. Analagous to the `Puzzle` structure.
struct TheoreticalState {
    value: Int<U>,
    order: Int<U>,
}

impl TheoreticalState {
    fn add_to_i(&mut self, amt: Int<I>) {
        self.add_to(amt % self.order);
    }

    fn add_to(&mut self, amt: Int<U>) {
        assert!(amt < self.order);

        self.value += amt % self.order;

        if self.value >= self.order {
            self.value -= self.order;
        }
    }
}

pub trait PuzzleState {
    /// Initialize the `Puzzle` in the solved state
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self;

    fn compose_into(&mut self, alg: &Algorithm);

    /// Check whether the given facelets are solved
    fn facelets_solved(&self, facelets: &[usize]) -> bool;

    /// Decode the permutation using the register generator and the given facelets.
    ///
    /// In general, an arbitrary scramble cannot be decoded. If this is the case, the function will return `None`.
    ///
    /// This function should not alter the cube state unless it returns `None`.
    fn print(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>>;

    /// Decode the register without requiring the cube state to be unaltered.
    fn halt(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        self.print(facelets, generator)
    }
}

#[derive(Clone, Debug)]
pub struct SimulatedPuzzle {
    perm_group: Arc<PermutationGroup>,
    state: Permutation,
}

impl SimulatedPuzzle {
    /// Get the state underlying the puzzle
    pub fn puzzle_state(&self) -> &Permutation {
        &self.state
    }
}

impl PuzzleState for SimulatedPuzzle {
    fn initialize(perm_group: Arc<PermutationGroup>) -> Self {
        SimulatedPuzzle {
            state: perm_group.identity(),
            perm_group,
        }
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        self.state.compose_into(alg.permutation());
    }

    fn facelets_solved(&self, facelets: &[usize]) -> bool {
        for &facelet in facelets {
            let maps_to = self.state.mapping()[facelet];
            if self.perm_group.facelet_colors()[maps_to]
                != self.perm_group.facelet_colors()[facelet]
            {
                return false;
            }
        }

        true
    }

    fn print(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        decode(&self.state, facelets, generator)
    }
}

/// A collection of the states of every puzzle and theoretical register
struct PuzzleStates<P: PuzzleState> {
    theoretical_states: Vec<TheoreticalState>,
    puzzle_states: Vec<P>,
}

pub struct InterpreterState<P: PuzzleState> {
    puzzle_states: PuzzleStates<P>,
    program_counter: usize,
    messages: VecDeque<String>,
    execution_state: ExecutionState,
}

/// An interpreter for a qter program
pub struct Interpreter<P: PuzzleState> {
    state: InterpreterState<P>,
    program: Program,
}

pub struct FaceletsByType;

impl SeparatesByPuzzleType for FaceletsByType {
    type Theoretical<'s> = ();

    type Puzzle<'s> = &'s Facelets;
}

pub struct AddAction {
    pub puzzle_idx: usize,
}

impl SeparatesByPuzzleType for AddAction {
    type Theoretical<'s> = (Self, Int<U>);

    type Puzzle<'s> = (Self, &'s Algorithm);
}

/// The action performed by the instruction that was just executed
pub enum ActionPerformed<'s> {
    None,
    Paused,
    Goto {
        instruction_idx: usize,
    },
    FailedSolvedGoto {
        puzzle_idx: usize,
        facelets: ByPuzzleType<'s, FaceletsByType>,
    },
    SucceededSolvedGoto {
        puzzle_idx: usize,
        instruction_idx: usize,
        facelets: ByPuzzleType<'s, FaceletsByType>,
    },
    Added(ByPuzzleType<'s, AddAction>),
    Panicked,
}

impl<P: PuzzleState> PuzzleStates<P> {
    /// Compose a permutation into a puzzle state
    fn compose_into(&mut self, puzzle_idx: usize, alg: &Algorithm) {
        // self.puzzle_states[puzzle_idx].state.compose(permutation);
        self.puzzle_states[puzzle_idx].compose_into(alg);
    }
}

impl<P: PuzzleState> InterpreterState<P> {
    /// Return the instruction index to be executed next
    #[must_use]
    pub fn program_counter(&self) -> usize {
        self.program_counter
    }

    /// Get the current execution state of the interpreter
    #[must_use]
    pub fn execution_state(&self) -> &ExecutionState {
        &self.execution_state
    }

    /// Get the message queue of the interpreter
    pub fn messages(&mut self) -> &mut VecDeque<String> {
        &mut self.messages
    }

    fn panic<'x>(&mut self, message: &str) -> ActionPerformed<'x> {
        self.execution_state = ExecutionState::Paused(PausedState::Panicked);
        self.messages.push_back(format!("Panicked: {message}"));
        ActionPerformed::Panicked
    }
}

impl<P: PuzzleState> Interpreter<P> {
    /// Get the program currently being executed
    #[must_use]
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Get the current state of the interpreter
    #[must_use]
    pub fn state(&self) -> &InterpreterState<P> {
        &self.state
    }

    /// Get the current state of the interpreter mutably
    #[must_use]
    pub fn state_mut(&mut self) -> &mut InterpreterState<P> {
        &mut self.state
    }

    /// Create a new interpreter from a program and initial states for registers
    ///
    /// If an initial state isn't specified, it defaults to zero.
    #[must_use]
    pub fn new(program: Program) -> Self {
        let theoretical_states = program
            .theoretical
            .iter()
            .map(|order| TheoreticalState {
                value: Int::zero(),
                order: **order,
            })
            .collect();

        let puzzle_states = program
            .puzzles
            .iter()
            .map(|perm_group| P::initialize(Arc::clone(perm_group)))
            .collect();

        let state = InterpreterState {
            puzzle_states: PuzzleStates {
                theoretical_states,
                puzzle_states,
            },
            program_counter: 0,
            messages: VecDeque::new(),
            execution_state: ExecutionState::Running,
        };

        Interpreter { state, program }
    }

    /// Execute one instruction
    pub fn step(&mut self) -> ActionPerformed<'_> {
        if let ExecutionState::Paused(_) = self.state.execution_state() {
            return ActionPerformed::Paused;
        }
        let Some(instruction) = self.program.instructions.get(self.state.program_counter) else {
            return self.state.panic(
                "Execution fell through the end of the program without reaching a halt instruction!"
            );
        };

        match &**instruction {
            &Instruction::Goto { instruction_idx } => {
                self.state.program_counter = instruction_idx;
                self.state.execution_state = ExecutionState::Running;

                ActionPerformed::Goto { instruction_idx }
            }
            Instruction::SolvedGoto(instr) => do_instr(instr, &mut self.state),
            Instruction::Input(instr) => do_instr(instr, &mut self.state),
            Instruction::Halt(instr) => do_instr(instr, &mut self.state),
            Instruction::Print(instr) => do_instr(instr, &mut self.state),
            Instruction::PerformAlgorithm(instr) => do_instr(instr, &mut self.state),
        }
    }

    /// Execute instructions until an input or halt instruction is reached
    ///
    /// Returns details of the paused state reached
    ///
    /// # Panics
    ///
    /// Panics if the interpreter is not in a paused state
    pub fn step_until_halt(&mut self) -> &PausedState {
        loop {
            if let ActionPerformed::Paused | ActionPerformed::Panicked = self.step() {
                break;
            }
        }
        match self.state.execution_state() {
            ExecutionState::Paused(v) => v,
            ExecutionState::Running => panic!("Cannot be halted while running"),
        }
    }

    /// Give an input to the interpreter, returning the puzzle index and the algorithm performed `value` times if applicable
    ///
    /// # Errors
    ///
    /// Returns an error if the input is out of bounds
    ///
    /// # Panics
    ///
    /// Panics if the interpreter is not executing an `input` instruction
    pub fn give_input(&mut self, value: Int<I>) -> Result<(usize, Option<Algorithm>), String> {
        let &ExecutionState::Paused(PausedState::Input {
            max_input,
            puzzle_idx: _,
            register: _,
        }) = &self.state.execution_state
        else {
            panic!("The interpreter isn't in an input state");
        };

        if value > max_input {
            return Err(format!("Your input must not be greater than {max_input}."));
        }
        if value < -max_input {
            return Err(format!("Your input must not be less than {}.", -max_input));
        }

        // The code is weird to appease the borrow checker

        let ExecutionState::Paused(PausedState::Input {
            max_input: _,
            puzzle_idx,
            register,
        }) = mem::replace(&mut self.state.execution_state, ExecutionState::Running)
        else {
            unreachable!("Checked before")
        };

        let exponentiated_alg = match register {
            ByPuzzleType::Theoretical(()) => {
                self.state.puzzle_states.theoretical_states[puzzle_idx].add_to_i(value);

                None
            }
            ByPuzzleType::Puzzle((mut algorithm, _)) => {
                let puzzle = &mut self.state.puzzle_states.puzzle_states[puzzle_idx];
                algorithm.exponentiate(value);

                puzzle.compose_into(&algorithm);

                Some(algorithm)
            }
        };

        self.state.execution_state = ExecutionState::Running;
        self.state.program_counter += 1;

        Ok((puzzle_idx, exponentiated_alg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Interpreter, PausedState};
    use compiler::compile;
    use internment::ArcIntern;
    use qter_core::{Int, U, architectures::PuzzleDefinition};
    use std::sync::Arc;

    #[test]
    fn facelets_solved() {
        let perm_group = Arc::new(
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap(),
        );

        let mut cube: SimulatedPuzzle =
            SimulatedPuzzle::initialize(Arc::clone(&perm_group.perm_group));

        // Remember that the decoder will subtract the smallest facelet found in the definition to make it zero based
        assert!(cube.facelets_solved(&[0, 8, 16, 24]));

        perm_group
            .perm_group
            .compose_generators_into(&mut cube.state, [ArcIntern::from("U")].iter())
            .unwrap();

        assert!(cube.facelets_solved(&[0, 12, 15, 7, 40]));

        assert!(!cube.facelets_solved(&[1, 12, 15, 7, 24]));
    }

    #[test]
    fn complicated_solved_decode_test() {
        let perm_group = Arc::new(
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap(),
        );

        let arch = perm_group
            .get_preset(&[Int::from(210_u64), Int::from(24_u64)])
            .unwrap();

        let a_facelets = arch.registers()[0].signature_facelets();
        let b_facelets = arch.registers()[1].signature_facelets();

        let a_permutation = Algorithm::new_from_effect(&arch, vec![(0, Int::one())]);
        let b_permutation = Algorithm::new_from_effect(&arch, vec![(1, Int::one())]);

        let mut cube: SimulatedPuzzle =
            SimulatedPuzzle::initialize(Arc::clone(&perm_group.perm_group));

        for i in 1..=23 {
            cube.state.compose_into(b_permutation.permutation());
            assert_eq!(
                cube.print(&b_facelets.0, &b_permutation).unwrap(),
                Int::from(i)
            );
            assert!(!cube.facelets_solved(&b_facelets.0));
        }

        cube.state.compose_into(b_permutation.permutation());
        assert!(cube.facelets_solved(&b_facelets.0));
        assert_eq!(
            cube.print(&b_facelets.0, &b_permutation).unwrap(),
            Int::<U>::zero()
        );

        for i in 0..24 {
            println!("{i}");
            for j in 0..210 {
                assert_eq!(
                    cube.print(&b_facelets.0, &b_permutation).unwrap(),
                    Int::from(i)
                );
                assert_eq!(
                    cube.print(&a_facelets.0, &a_permutation).unwrap(),
                    Int::from(j)
                );

                cube.state.compose_into(a_permutation.permutation());
            }

            cube.state.compose_into(b_permutation.permutation());
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

        let mut interpreter: Interpreter<SimulatedPuzzle> = Interpreter::new(program);

        assert!(match interpreter.step_until_halt() {
            PausedState::Input {
                max_input,
                register: ByPuzzleType::Puzzle(_),
                puzzle_idx: 0,
            } => *max_input == Int::from(209),
            _ => false,
        });

        assert!(interpreter.give_input(Int::from(133_u64)).is_ok());

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                maybe_puzzle_idx_and_register: Some((0, ByPuzzleType::Puzzle(_))),
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
            interpreter.state_mut().messages().len(),
            "{:?}",
            interpreter.state_mut().messages()
        );

        for (message, expected) in interpreter
            .state()
            .messages
            .iter()
            .zip(expected_output.iter())
        {
            assert_eq!(message, expected);
        }
    }

    #[test]
    fn modulus_2() {
        let code = "
            .registers {
                A, B ← 3x3 builtin (90, 90)
            }

                input \"Number to modulus:\" A
            loop:
                print \"A is now\" A
                solved-goto A%9 finalize
                add B 1
                add A 89
                goto loop
            finalize:
                halt \"The modulus is\" B
        ";

        let program = match compile(code, |_| unreachable!()) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };

        let mut interpreter: Interpreter<SimulatedPuzzle> = Interpreter::new(program);

        let halted_state = interpreter.step_until_halt();
        assert!(
            match halted_state {
                PausedState::Input {
                    max_input,
                    register: ByPuzzleType::Puzzle(_),
                    puzzle_idx: 0,
                } => *max_input == Int::from(89),
                _ => false,
            },
            "{halted_state:?}"
        );

        assert!(interpreter.give_input(Int::from(77_u64)).is_ok());

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                maybe_puzzle_idx_and_register: Some((0, ByPuzzleType::Puzzle(_))),
            }
        ));

        let expected_output = [
            "Number to modulus: (max input 89)",
            "A is now 77",
            "A is now 76",
            "A is now 75",
            "A is now 74",
            "A is now 73",
            "A is now 72",
            "The modulus is 5",
        ];

        assert_eq!(
            expected_output.len(),
            interpreter.state_mut().messages().len(),
            "{:?}",
            interpreter.state_mut().messages()
        );

        for (message, expected) in interpreter
            .state()
            .messages
            .iter()
            .zip(expected_output.iter())
        {
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

        let mut interpreter: Interpreter<SimulatedPuzzle> = Interpreter::new(program);

        assert!(match interpreter.step_until_halt() {
            PausedState::Input {
                max_input,
                register: ByPuzzleType::Puzzle(_),
                puzzle_idx: 0,
            } => *max_input == Int::from(8),
            _ => false,
        });

        assert!(interpreter.give_input(Int::from(8_u64)).is_ok());

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                maybe_puzzle_idx_and_register: Some((0, ByPuzzleType::Puzzle(_))),
            }
        ));

        let expected_output = [
            "Which Fibonacci number to calculate: (max input 8)",
            "The number is 21",
        ];

        assert_eq!(
            expected_output.len(),
            interpreter.state_mut().messages().len(),
            "{:?}",
            interpreter.state_mut().messages()
        );

        for (message, expected) in interpreter
            .state()
            .messages
            .iter()
            .zip(expected_output.iter())
        {
            assert_eq!(message, expected);
        }
    }
}
