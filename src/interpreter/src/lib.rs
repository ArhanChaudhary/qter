use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use bnum::types::U512;
use qter_core::{
    architectures::{Architecture, Permutation},
    Instruction, PermuteCube, Program, RegisterReference, Span,
};

pub struct Puzzle {
    architecture: Rc<Architecture>,
    state: Permutation,
}

impl Puzzle {
    pub fn initialize(architecture: Rc<Architecture>) -> Puzzle {
        Puzzle {
            state: architecture.group().identity(),
            architecture,
        }
    }

    pub fn architecture(&self) -> &Architecture {
        &self.architecture
    }

    pub fn state(&self) -> &Permutation {
        &self.state
    }
}

pub enum PausedState<'s> {
    Halt {
        message: &'s str,
        register: RegisterReference,
    },
    Input {
        message: &'s str,
        register: RegisterReference,
    },
}

pub enum StateTy<'s> {
    Running,
    Paused(PausedState<'s>),
}

///  The current execution state of the interpreter
pub struct State<'s> {
    span: Span,
    instruction: usize,
    state_ty: StateTy<'s>,
}

struct TheoreticalState {
    state: U512,
    order: U512,
}

struct GroupStates {
    theoretical_states: Vec<TheoreticalState>,
    puzzle_states: Vec<Puzzle>,
}

/// Interprets a decoded qter program
pub struct Interpreter {
    group_state: GroupStates,
    messages: VecDeque<String>,
    instruction_counter: usize,
    program: Program,
    paused: bool,
}

pub enum ActionPerformed {
    None,
    Paused,
    Goto {
        location: usize,
    },
    FailedSolvedGoto {
        register: RegisterReference,
    },
    SucceededSolvedGoto {
        register: RegisterReference,
        location: usize,
    },
    AddToTheoretical {
        register: usize,
        amt: U512,
    },
    ExecutedAlgorithm(PermuteCube),
}

impl GroupStates {
    fn is_register_solved(&self, which_reg: RegisterReference) -> bool {
        match which_reg {
            RegisterReference::Theoretical { idx } => self.theoretical_states[idx].state.is_zero(),
            RegisterReference::Puzzle {
                idx,
                which_register,
            } => {
                let puzzle = &self.puzzle_states[idx];
                puzzle.architecture.registers()[which_register].is_solved(&puzzle.state)
            }
        }
    }

    fn decode_register(&self, which_reg: RegisterReference) -> U512 {
        match which_reg {
            RegisterReference::Theoretical { idx } => self.theoretical_states[idx].state,
            RegisterReference::Puzzle {
                idx,
                which_register,
            } => {
                let puzzle = &self.puzzle_states[idx];
                puzzle.architecture.registers()[which_register].decode(&puzzle.state)
            }
        }
    }

    fn add_num_to(&mut self, which_reg: RegisterReference, amt: U512) {
        match which_reg {
            RegisterReference::Theoretical { idx } => {
                let TheoreticalState { state, order } = &mut self.theoretical_states[idx];

                assert!(amt < *order);

                *state += amt;

                if *state >= *order {
                    *state -= *order;
                }
            }
            RegisterReference::Puzzle {
                idx,
                which_register,
            } => {
                let puzzle = &mut self.puzzle_states[idx];
                let mut perm = puzzle.architecture.registers()[which_register]
                    .permutation()
                    .to_owned();

                perm.exponentiate(amt);

                puzzle.state.compose(&perm);
            }
        }
    }

    fn compose_into(&mut self, puzzle_idx: usize, permutation: &Permutation) {
        self.puzzle_states[puzzle_idx].state.compose(permutation);
    }
}

impl Interpreter {
    /// Create a new interpreter from a program and initial states for registers
    ///
    /// If an initial state isn't specified, it defaults to zero.
    pub fn new(
        program: Program,
        mut initial_states: HashMap<RegisterReference, U512>,
    ) -> Interpreter {
        let theoretical_states = program
            .theoretical
            .iter()
            .enumerate()
            .map(|(i, order)| TheoreticalState {
                state: initial_states
                    .remove(&RegisterReference::Theoretical { idx: i })
                    .unwrap_or(U512::ZERO),
                order: **order,
            })
            .collect();

        let puzzle_states = program
            .puzzles
            .iter()
            .map(|arch| Puzzle::initialize(Rc::clone(arch)))
            .collect();

        assert_eq!(initial_states.len(), 0);

        Interpreter {
            group_state: GroupStates {
                theoretical_states,
                puzzle_states,
            },
            program,
            instruction_counter: 0,
            paused: false,
            messages: VecDeque::new(),
        }
    }

    /// Get the current state of the interpreter
    pub fn state(&self) -> State<'_> {
        let instruction = &self.program.instructions[self.instruction_counter];

        let state_ty = match &**instruction {
            Instruction::Halt { message, register } => StateTy::Paused(PausedState::Halt {
                message,
                register: *register,
            }),
            Instruction::Input { message, register } => StateTy::Paused(PausedState::Input {
                message,
                register: *register,
            }),
            _ => StateTy::Running,
        };

        State {
            span: instruction.span().to_owned(),
            instruction: self.instruction_counter,
            state_ty,
        }
    }

    /// Execute one instruction
    pub fn step(&mut self) -> ActionPerformed {
        println!("PC: {}", self.instruction_counter);

        let instruction = &self.program.instructions[self.instruction_counter];

        match &**instruction {
            Instruction::Goto { instruction_idx } => {
                self.instruction_counter = *instruction_idx;

                ActionPerformed::Goto {
                    location: *instruction_idx,
                }
            }
            Instruction::SolvedGoto {
                instruction_idx,
                register,
            } => {
                if self.group_state.is_register_solved(*register) {
                    self.instruction_counter = *instruction_idx;

                    ActionPerformed::SucceededSolvedGoto {
                        register: register.to_owned(),
                        location: *instruction_idx,
                    }
                } else {
                    self.instruction_counter += 1;

                    ActionPerformed::FailedSolvedGoto {
                        register: register.to_owned(),
                    }
                }
            }
            Instruction::Input {
                message,
                register: _,
            } => {
                if !self.paused {
                    self.paused = true;
                    self.messages.push_back(message.to_owned());
                }

                ActionPerformed::Paused
            }
            Instruction::Halt { message, register } => {
                if !self.paused {
                    self.paused = true;
                    self.messages.push_back(format!(
                        "{message} {}",
                        self.group_state.decode_register(*register)
                    ));
                }

                ActionPerformed::Paused
            }
            Instruction::Print { message, register } => {
                self.messages.push_back(format!(
                    "{message} {}",
                    self.group_state.decode_register(*register)
                ));

                self.instruction_counter += 1;

                ActionPerformed::None
            }
            Instruction::AddTheoretical { register, amount } => {
                let reg = RegisterReference::Theoretical { idx: *register };

                self.group_state.add_num_to(reg, *amount);
                self.instruction_counter += 1;

                ActionPerformed::AddToTheoretical {
                    register: *register,
                    amt: *amount,
                }
            }
            Instruction::PermuteCube(permute_cube) => {
                self.group_state
                    .compose_into(permute_cube.cube_idx(), permute_cube.permutation());

                self.instruction_counter += 1;

                ActionPerformed::ExecutedAlgorithm(permute_cube.to_owned())
            }
        }
    }

    /// Execute instructions until an input or halt instruction is reached
    ///
    /// Returns details of the paused state reached
    pub fn step_until_halt(&mut self) -> PausedState<'_> {
        while !self.paused {
            self.step();
        }

        match self.state().state_ty {
            StateTy::Running => panic!("Cannot be halted while running"),
            StateTy::Paused(v) => v,
        }
    }

    /// Give an input to the interpreter
    ///
    /// Panics if the interpreter is not executing an `input` instruction
    pub fn give_input(&mut self, value: U512) {
        let reg = match self.state().state_ty {
            StateTy::Paused(PausedState::Input {
                message: _,
                register,
            }) => register.to_owned(),
            _ => panic!("The interpreter isn't in an input state"),
        };

        self.group_state.add_num_to(reg, value);

        self.paused = false;
        self.instruction_counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, rc::Rc};

    use bnum::types::U512;
    use qter_core::{
        architectures::PuzzleDefinition, Instruction, PermuteCube, Program, RegisterReference,
        Span, WithSpan,
    };

    use crate::{Interpreter, PausedState};

    #[test]
    fn modulus() {
        /*
            input Number to modulus: A
        loop:
            print A is now A
            B += N
        decrement:
            solved-goto B loop
            solved-goto A fix
            A -= 1
            B -= 1
            goto decrement
        fix:
            solved-goto B finalize
            A -= 1
            B -= 1
            goto fix
        finalize:
            A += N
            halt The modulus is A
        */

        let random_span = Span::new(Rc::from("bruh"), 0, 0);

        let cube =
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap();

        let arch = cube
            .get_preset(&[U512::from_digit(24), U512::from_digit(210)])
            .unwrap();

        // Define the registers
        let puzzles = vec![WithSpan::new(Rc::clone(&arch), random_span.to_owned())];

        let a = RegisterReference::Puzzle {
            idx: 0,
            which_register: 1,
        };
        let b = RegisterReference::Puzzle {
            idx: 0,
            which_register: 0,
        };

        let a_idx = 1;
        let b_idx = 0;

        let to_modulus = U512::from_digit(13);
        // Negative numbers by overflowing
        let a_minus_1 = U512::from_digit(209);
        let b_minus_1 = U512::from_digit(23);

        let instructions = vec![
            // 0
            Instruction::Input {
                message: "Number to modulus:".to_owned(),
                register: a,
            },
            // 1; loop:
            Instruction::Print {
                message: "A is now".to_owned(),
                register: a,
            },
            // 2
            Instruction::PermuteCube(PermuteCube::new(&arch, 0, vec![(b_idx, to_modulus)])),
            // 3; decrement:
            Instruction::SolvedGoto {
                instruction_idx: 1, // loop
                register: b,
            },
            // 4
            Instruction::SolvedGoto {
                instruction_idx: 7, // fix
                register: a,
            },
            // 5
            Instruction::PermuteCube(PermuteCube::new(
                &arch,
                0,
                vec![(a_idx, a_minus_1), (b_idx, b_minus_1)],
            )),
            // 6
            Instruction::Goto { instruction_idx: 3 }, // decrement
            // 7; fix:
            Instruction::SolvedGoto {
                instruction_idx: 10, // finalize
                register: b,
            },
            // 8
            Instruction::PermuteCube(PermuteCube::new(
                &arch,
                0,
                vec![(a_idx, a_minus_1), (b_idx, b_minus_1)],
            )),
            // 9
            Instruction::Goto { instruction_idx: 7 }, // fix
            // 10; finalize:
            Instruction::PermuteCube(PermuteCube::new(&arch, 0, vec![(a_idx, to_modulus)])),
            // 11
            Instruction::Halt {
                message: "The modulus is".to_owned(),
                register: a,
            },
        ];

        let program = Program {
            instructions: instructions
                .into_iter()
                .map(|v| WithSpan::new(v, random_span.to_owned()))
                .collect(),
            theoretical: vec![],
            puzzles,
        };

        let mut interpreter = Interpreter::new(program, HashMap::new());

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Input {
                message: "Number to modulus:",
                register: RegisterReference::Puzzle {
                    idx: 0,
                    which_register: 1
                },
            }
        ));

        interpreter.give_input(U512::from_digit(133));

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                message: "The modulus is",
                register: RegisterReference::Puzzle {
                    idx: 0,
                    which_register: 1
                },
            }
        ));

        let expected_output = [
            "Number to modulus:",
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
            interpreter.messages.len(),
            "{:?}",
            interpreter.messages
        );

        for (message, expected) in interpreter.messages.iter().zip(expected_output.iter()) {
            assert_eq!(message, expected);
        }
    }
}
