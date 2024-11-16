use std::collections::{HashMap, VecDeque};

use bnum::types::U512;
use qter_core::{Instruction, Program};

enum GroupState {
    Theoretical {
        name: String,
        value: U512,
        order: U512,
    },
}

pub enum PausedState<'s> {
    Halt { message: &'s str, register: &'s str },
    Input { message: &'s str, register: &'s str },
}

pub enum StateTy<'s> {
    Running,
    Paused(PausedState<'s>),
}

pub struct State<'s> {
    line: usize,
    instruction: usize,
    state_ty: StateTy<'s>,
}

pub struct Interpreter {
    group_states: Vec<GroupState>,
    messages: VecDeque<String>,
    instruction_counter: usize,
    program: Program,
    paused: bool,
}

impl Interpreter {
    pub fn new(program: Program, mut initial_states: HashMap<String, U512>) -> Interpreter {
        let mut group_states = Vec::with_capacity(program.groups.len());

        for group in &program.groups {
            match &**group {
                qter_core::RegisterRepresentation::Theoretical { name, order } => {
                    group_states.push(GroupState::Theoretical {
                        name: name.to_owned(),
                        value: initial_states.remove(name).unwrap_or(U512::from_digit(0)),
                        order: *order,
                    });
                }
            }
        }

        Interpreter {
            group_states,
            program,
            instruction_counter: 0,
            paused: false,
            messages: VecDeque::new(),
        }
    }

    pub fn state(&self) -> State<'_> {
        let instruction = &self.program.instructions[self.instruction_counter];

        let state_ty = match &**instruction {
            Instruction::Halt { message, register } => {
                StateTy::Paused(PausedState::Halt { message, register })
            }
            Instruction::Input { message, register } => {
                StateTy::Paused(PausedState::Input { message, register })
            }
            _ => StateTy::Running,
        };

        State {
            line: instruction.line_num(),
            instruction: self.instruction_counter,
            state_ty,
        }
    }

    fn register_solved(group_states: &[GroupState], reg: &str) -> bool {
        for group_state in group_states {
            match group_state {
                GroupState::Theoretical {
                    name,
                    value,
                    order: _,
                } => {
                    if name == reg {
                        return value.is_zero();
                    }
                }
            }
        }

        panic!("Failed to find register {reg}!");
    }

    fn decode_register(group_states: &[GroupState], reg: &str) -> U512 {
        for group_state in group_states {
            match group_state {
                GroupState::Theoretical {
                    name,
                    value,
                    order: _,
                } => {
                    if name == reg {
                        return *value;
                    }
                }
            }
        }

        panic!("Failed to find register {reg}!");
    }

    fn add_num_to(group_states: &mut [GroupState], reg: &str, amt: U512) {
        for group_state in group_states {
            match group_state {
                GroupState::Theoretical { name, value, order } => {
                    if name == reg {
                        assert!(amt < *order);

                        *value += amt;

                        if value >= order {
                            *value -= *order;
                        }

                        return;
                    }
                }
            }
        }

        panic!("Failed to find register {reg}!");
    }

    pub fn step(&mut self) {
        let instruction = &self.program.instructions[self.instruction_counter];

        match &**instruction {
            Instruction::Goto { instruction_idx } => {
                self.instruction_counter = *instruction_idx;
            }
            Instruction::SolvedGoto {
                instruction_idx,
                register,
            } => {
                if Self::register_solved(&self.group_states, register) {
                    self.instruction_counter = *instruction_idx;
                } else {
                    self.instruction_counter += 1;
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
            }
            Instruction::Halt { message, register } => {
                if !self.paused {
                    self.paused = true;
                    self.messages.push_back(format!(
                        "{message} {}",
                        Self::decode_register(&self.group_states, register)
                    ));
                }
            }
            Instruction::Print { message, register } => {
                self.messages.push_back(format!(
                    "{message} {}",
                    Self::decode_register(&self.group_states, register)
                ));

                self.instruction_counter += 1;
            }
            Instruction::AddTheoretical { register, amount } => {
                Self::add_num_to(&mut self.group_states, &register, *amount);
                self.instruction_counter += 1;
            }
        }
    }

    pub fn step_until_halt(&mut self) -> PausedState<'_> {
        while !self.paused {
            self.step();
        }

        match self.state().state_ty {
            StateTy::Running => panic!("Cannot be halted while running"),
            StateTy::Paused(v) => v,
        }
    }

    pub fn give_input(&mut self, value: U512) {
        let reg = match self.state().state_ty {
            StateTy::Paused(PausedState::Input {
                message: _,
                register,
            }) => register.to_owned(),
            _ => panic!("The interpreter isn't in an input state"),
        };

        Self::add_num_to(&mut self.group_states, &reg, value);

        self.paused = false;
        self.instruction_counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bnum::types::U512;
    use qter_core::{Instruction, Program, RegisterRepresentation, WithSpan};

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

        let groups = vec![
            WithSpan::new(
                RegisterRepresentation::Theoretical {
                    name: "A".to_owned(),
                    order: U512::from_digit(210),
                },
                0,
            ),
            WithSpan::new(
                RegisterRepresentation::Theoretical {
                    name: "B".to_owned(),
                    order: U512::from_digit(24),
                },
                0,
            ),
        ];

        let to_modulus = U512::from_digit(13);
        let a_minus_1 = U512::from_digit(209);
        let b_minus_1 = U512::from_digit(23);

        let instructions = vec![
            // 0
            Instruction::Input {
                message: "Number to modulus:".to_owned(),
                register: "A".to_owned(),
            },
            // 1; loop:
            Instruction::Print {
                message: "A is now".to_owned(),
                register: "A".to_owned(),
            },
            // 2
            Instruction::AddTheoretical {
                register: "B".to_owned(),
                amount: to_modulus,
            },
            // 3; decrement:
            Instruction::SolvedGoto {
                instruction_idx: 1, // loop
                register: "B".to_owned(),
            },
            // 4
            Instruction::SolvedGoto {
                instruction_idx: 8, // fix
                register: "A".to_owned(),
            },
            // 5
            Instruction::AddTheoretical {
                register: "A".to_owned(),
                amount: a_minus_1,
            },
            // 6
            Instruction::AddTheoretical {
                register: "B".to_owned(),
                amount: b_minus_1,
            },
            // 7
            Instruction::Goto { instruction_idx: 3 }, // decrement
            // 8; fix:
            Instruction::SolvedGoto {
                instruction_idx: 12, // finalize
                register: "B".to_owned(),
            },
            // 9
            Instruction::AddTheoretical {
                register: "A".to_owned(),
                amount: a_minus_1,
            },
            // 10
            Instruction::AddTheoretical {
                register: "B".to_owned(),
                amount: b_minus_1,
            },
            // 11
            Instruction::Goto { instruction_idx: 8 }, // fix
            // 12; finalize:
            Instruction::AddTheoretical {
                register: "A".to_owned(),
                amount: to_modulus,
            },
            // 13
            Instruction::Halt {
                message: "The modulus is".to_owned(),
                register: "A".to_owned(),
            },
        ];

        let program = Program {
            groups,
            instructions: instructions
                .into_iter()
                .map(|v| WithSpan::new(v, 0))
                .collect(),
        };

        let mut interpreter = Interpreter::new(program, HashMap::new());

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Input {
                message: "Number to modulus:",
                register: "A"
            }
        ));

        interpreter.give_input(U512::from_digit(133));

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                message: "The modulus is",
                register: "A"
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
