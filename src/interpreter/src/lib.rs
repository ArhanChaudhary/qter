use std::{collections::VecDeque, rc::Rc};

use bnum::types::U512;
use qter_core::{
    architectures::{Permutation, PermutationGroup},
    discrete_math::chinese_remainder_theorem,
    Facelets, Instruction, PermuteCube, Program, RegisterGenerator, Span,
};

pub struct Puzzle {
    group: Rc<PermutationGroup>,
    state: Permutation,
}

impl Puzzle {
    pub fn initialize(group: Rc<PermutationGroup>) -> Puzzle {
        Puzzle {
            state: group.identity(),
            group,
        }
    }

    pub fn facelets_solved(&self, facelets: &[usize]) -> bool {
        for facelet in facelets {
            let maps_to = self.state().mapping()[*facelet];
            println!(
                "{facelet}; {maps_to}: {}; {}",
                self.group.facelet_colors()[self.state().mapping()[*facelet]],
                self.group().facelet_colors()[*facelet],
            );
            if self.group.facelet_colors()[maps_to] != self.group.facelet_colors()[*facelet] {
                return false;
            }
        }

        true
    }

    pub fn decode(&self, facelets: &[usize], generator: &PermuteCube) -> Option<U512> {
        let mut constraints = Vec::new();

        for facelet in facelets {
            let maps_to = self.state().mapping()[*facelet];

            let chromatic_order = generator.chromatic_orders_by_facelets()[*facelet];

            if maps_to == *facelet {
                constraints.push((U512::ZERO, chromatic_order));
                continue;
            }

            let mut i = U512::ONE;
            let mut maps_to_found_at = None;
            let mut facelet_at = generator.permutation().mapping()[*facelet];

            while facelet_at != *facelet {
                if facelet_at == maps_to {
                    maps_to_found_at = Some(i);
                    break;
                }

                facelet_at = generator.permutation().mapping()[facelet_at];
                i += U512::ONE;
            }

            match maps_to_found_at {
                Some(found_at) => constraints.push((found_at % chromatic_order, chromatic_order)),
                None => return None,
            }
        }

        Some(chinese_remainder_theorem(constraints))
    }

    pub fn group(&self) -> &PermutationGroup {
        &self.group
    }

    pub fn state(&self) -> &Permutation {
        &self.state
    }
}

pub enum PausedState<'s> {
    Halt {
        message: &'s str,
        register_idx: usize,
        register: RegisterGenerator,
    },
    Input {
        message: &'s str,
        register_idx: usize,
        register: RegisterGenerator,
    },
    Panicked(String),
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
    panicked: Option<String>,
}

pub enum ActionPerformed {
    None,
    Paused,
    Goto {
        location: usize,
    },
    FailedSolvedGoto {
        register_idx: usize,
        facelets: Facelets,
    },
    SucceededSolvedGoto {
        register_idx: usize,
        facelets: Facelets,
        location: usize,
    },
    AddToTheoretical {
        register_idx: usize,
        amt: U512,
    },
    ExecutedAlgorithm(PermuteCube),
    Panic(String),
}

impl GroupStates {
    fn is_register_solved(&self, register_idx: usize, which_reg: &Facelets) -> bool {
        match which_reg {
            Facelets::Theoretical => self.theoretical_states[register_idx].state.is_zero(),
            Facelets::Puzzle { facelets } => {
                let puzzle = &self.puzzle_states[register_idx];
                puzzle.facelets_solved(facelets)
            }
        }
    }

    fn decode_register(&self, register_idx: usize, which_reg: &RegisterGenerator) -> Option<U512> {
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

    fn add_num_to(&mut self, register_idx: usize, which_reg: &RegisterGenerator, amt: U512) {
        match which_reg {
            RegisterGenerator::Theoretical => {
                let TheoreticalState { state, order } = &mut self.theoretical_states[register_idx];

                assert!(amt < *order);

                *state += amt;

                if *state >= *order {
                    *state -= *order;
                }
            }
            RegisterGenerator::Puzzle {
                generator,
                facelets: _,
            } => {
                let puzzle = &mut self.puzzle_states[register_idx];
                let mut perm = generator.permutation().to_owned();

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
    pub fn new(program: Program) -> Interpreter {
        let theoretical_states = program
            .theoretical
            .iter()
            .map(|order| TheoreticalState {
                state: U512::ZERO,
                order: **order,
            })
            .collect();

        let puzzle_states = program
            .puzzles
            .iter()
            .map(|group| Puzzle::initialize(Rc::clone(group)))
            .collect();

        Interpreter {
            group_state: GroupStates {
                theoretical_states,
                puzzle_states,
            },
            program,
            instruction_counter: 0,
            paused: false,
            messages: VecDeque::new(),
            panicked: None,
        }
    }

    /// Get the current state of the interpreter
    pub fn state(&self) -> State<'_> {
        let instruction = &self.program.instructions[self.instruction_counter];

        let state_ty = if let Some(message) = &self.panicked {
            StateTy::Paused(PausedState::Panicked(message.to_owned()))
        } else {
            match &**instruction {
                Instruction::Halt {
                    message,
                    register,
                    register_idx,
                } => StateTy::Paused(PausedState::Halt {
                    message,
                    register: register.to_owned(),
                    register_idx: *register_idx,
                }),
                Instruction::Input {
                    message,
                    register,
                    register_idx,
                } => StateTy::Paused(PausedState::Input {
                    message,
                    register: register.to_owned(),
                    register_idx: *register_idx,
                }),
                _ => StateTy::Running,
            }
        };

        State {
            span: instruction.span().to_owned(),
            instruction: self.instruction_counter,
            state_ty,
        }
    }

    fn panic(&mut self, message: String) -> ActionPerformed {
        self.panicked = Some(message.to_owned());
        self.paused = true;
        self.messages.push_back(format!("Panicked: {message}"));
        ActionPerformed::Panic(message)
    }

    /// Execute one instruction
    pub fn step(&mut self) -> ActionPerformed {
        // println!("PC: {}", self.instruction_counter);

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
                facelets,
                register_idx,
            } => {
                if self.group_state.is_register_solved(*register_idx, facelets) {
                    self.instruction_counter = *instruction_idx;

                    ActionPerformed::SucceededSolvedGoto {
                        facelets: facelets.to_owned(),
                        location: *instruction_idx,
                        register_idx: *register_idx,
                    }
                } else {
                    self.instruction_counter += 1;

                    ActionPerformed::FailedSolvedGoto {
                        facelets: facelets.to_owned(),
                        register_idx: *register_idx,
                    }
                }
            }
            Instruction::Input {
                message,
                register: _,
                register_idx: _,
            } => {
                if !self.paused {
                    self.paused = true;
                    self.messages.push_back(message.to_owned());
                }

                ActionPerformed::Paused
            }
            Instruction::Halt {
                message,
                register,
                register_idx,
            } => {
                let decoded = match self.group_state.decode_register(*register_idx, register) {
                    Some(v) => v,
                    None => {
                        return self.panic("The register specified is not decodable!".to_owned());
                    }
                };

                if !self.paused {
                    self.paused = true;
                    self.messages.push_back(format!("{message} {decoded}",));
                }

                ActionPerformed::Paused
            }
            Instruction::Print {
                message,
                register,
                register_idx,
            } => {
                let decoded = match self.group_state.decode_register(*register_idx, register) {
                    Some(v) => v,
                    None => {
                        return self.panic("The register specified is not decodable!".to_owned());
                    }
                };

                self.messages.push_back(format!("{message} {decoded}"));

                self.instruction_counter += 1;

                ActionPerformed::None
            }
            Instruction::AddTheoretical {
                register_idx,
                amount,
            } => {
                let reg = RegisterGenerator::Theoretical;

                self.group_state.add_num_to(*register_idx, &reg, *amount);

                self.instruction_counter += 1;

                ActionPerformed::AddToTheoretical {
                    register_idx: *register_idx,
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
                register_idx,
            }) => (register_idx, register),
            _ => panic!("The interpreter isn't in an input state"),
        };

        self.group_state.add_num_to(reg.0, &reg.1, value);

        self.paused = false;
        self.instruction_counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use bnum::types::U512;
    use internment::ArcIntern;
    use qter_core::{
        architectures::PuzzleDefinition, Facelets, Instruction, PermuteCube, Program,
        RegisterGenerator, Span, WithSpan,
    };

    use crate::{Interpreter, PausedState, Puzzle};

    #[test]
    fn facelets_solved() {
        let group = Rc::new(
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap(),
        );

        let mut cube = Puzzle::initialize(Rc::clone(&group.group));

        // Remember that the decoder will subtract the smallest facelet found in the definition to make it zero based
        assert!(cube.facelets_solved(&[0, 8, 16, 24]));

        group
            .group
            .compose_generators_into(&mut cube.state, [ArcIntern::from_ref("U")].iter())
            .unwrap();

        assert!(cube.facelets_solved(&[0, 12, 15, 7, 40]));

        assert!(!cube.facelets_solved(&[1, 12, 15, 7, 24]));
    }

    #[test]
    fn decode() {
        let group = Rc::new(
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap(),
        );

        let mut cube = Puzzle::initialize(Rc::clone(&group.group));

        let permutation = PermuteCube::new_from_generators(
            Rc::clone(&group.group),
            0,
            vec![ArcIntern::from_ref("U")],
        )
        .unwrap();

        assert_eq!(cube.decode(&[8], &permutation).unwrap(), U512::ZERO);
        assert!(cube.facelets_solved(&[8]));

        cube.state.compose(permutation.permutation());
        assert_eq!(cube.decode(&[8], &permutation).unwrap(), U512::ONE);
        assert!(!cube.facelets_solved(&[8]));

        cube.state.compose(permutation.permutation());
        assert_eq!(cube.decode(&[8], &permutation).unwrap(), U512::TWO);
        assert!(!cube.facelets_solved(&[8]));

        cube.state.compose(permutation.permutation());
        assert_eq!(cube.decode(&[8], &permutation).unwrap(), U512::THREE);
        assert!(!cube.facelets_solved(&[8]));

        cube.state.compose(permutation.permutation());
        assert_eq!(cube.decode(&[8], &permutation).unwrap(), U512::ZERO);
        assert!(cube.facelets_solved(&[8]));
    }

    #[test]
    fn complicated_solved_decode_test() {
        let group = Rc::new(
            PuzzleDefinition::parse(include_str!("../../qter_core/puzzles/3x3.txt")).unwrap(),
        );

        let arch = group
            .get_preset(&[U512::from_digit(210), U512::from_digit(24)])
            .unwrap();

        let a_facelets = arch.registers()[0].signature_facelets();
        let b_facelets = arch.registers()[1].signature_facelets();

        println!("{b_facelets:?}");

        let a_permutation = PermuteCube::new_from_effect(&arch, 0, vec![(0, U512::ONE)]);
        let b_permutation = PermuteCube::new_from_effect(&arch, 0, vec![(1, U512::ONE)]);

        let mut cube = Puzzle::initialize(Rc::clone(&group.group));

        for i in 1..=23 {
            cube.state.compose(b_permutation.permutation());
            assert_eq!(
                cube.decode(&b_facelets, &b_permutation).unwrap(),
                U512::from_digit(i)
            );
            assert!(!cube.facelets_solved(&b_facelets));
        }

        cube.state.compose(b_permutation.permutation());
        assert!(cube.facelets_solved(&b_facelets));
        assert_eq!(
            cube.decode(&b_facelets, &b_permutation).unwrap(),
            U512::ZERO
        );

        for i in 0..24 {
            for j in 0..210 {
                assert_eq!(
                    cube.decode(&b_facelets, &b_permutation).unwrap(),
                    U512::from_digit(i)
                );
                assert_eq!(
                    cube.decode(&a_facelets, &a_permutation).unwrap(),
                    U512::from_digit(j)
                );

                cube.state.compose(a_permutation.permutation());
            }

            cube.state.compose(b_permutation.permutation());
        }
    }

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
        let puzzles = vec![WithSpan::new(arch.group_rc(), random_span.to_owned())];

        let a_facelets = arch.registers()[1].signature_facelets();
        let b_facelets = arch.registers()[0].signature_facelets();

        let a_gen = RegisterGenerator::Puzzle {
            generator: PermuteCube::new_from_effect(&arch, 0, vec![(1, U512::ONE)]),
            facelets: a_facelets.to_owned(),
        };
        let b_gen = RegisterGenerator::Puzzle {
            generator: PermuteCube::new_from_effect(&arch, 0, vec![(0, U512::ONE)]),
            facelets: b_facelets.to_owned(),
        };

        let a_fl = Facelets::Puzzle {
            facelets: a_facelets,
        };
        let b_fl = Facelets::Puzzle {
            facelets: b_facelets.to_owned(),
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
                register: a_gen.to_owned(),
                register_idx: 0,
            },
            // 1; loop:
            Instruction::Print {
                message: "A is now".to_owned(),
                register: a_gen.to_owned(),
                register_idx: 0,
            },
            // 2
            Instruction::PermuteCube(PermuteCube::new_from_effect(
                &arch,
                0,
                vec![(b_idx, to_modulus)],
            )),
            // 3; decrement:
            Instruction::SolvedGoto {
                instruction_idx: 1, // loop
                facelets: b_fl.to_owned(),
                register_idx: 0,
            },
            // 4
            Instruction::SolvedGoto {
                instruction_idx: 7, // fix
                facelets: a_fl.to_owned(),
                register_idx: 0,
            },
            // 5
            Instruction::PermuteCube(PermuteCube::new_from_effect(
                &arch,
                0,
                vec![(a_idx, a_minus_1), (b_idx, b_minus_1)],
            )),
            // 6
            Instruction::Goto { instruction_idx: 3 }, // decrement
            // 7; fix:
            Instruction::SolvedGoto {
                instruction_idx: 10, // finalize
                facelets: b_fl.to_owned(),
                register_idx: 0,
            },
            // 8
            Instruction::PermuteCube(PermuteCube::new_from_effect(
                &arch,
                0,
                vec![(a_idx, a_minus_1), (b_idx, b_minus_1)],
            )),
            // 9
            Instruction::Goto { instruction_idx: 7 }, // fix
            // 10; finalize:
            Instruction::PermuteCube(PermuteCube::new_from_effect(
                &arch,
                0,
                vec![(a_idx, to_modulus)],
            )),
            // 11
            Instruction::Halt {
                message: "The modulus is".to_owned(),
                register: a_gen.to_owned(),
                register_idx: 0,
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

        let mut interpreter = Interpreter::new(program);

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Input {
                message: "Number to modulus:",
                register: RegisterGenerator::Puzzle {
                    generator: _,
                    facelets: _
                },
                register_idx: 0,
            }
        ));

        interpreter.give_input(U512::from_digit(133));

        // for _ in 0..1000 {
        //     if interpreter.paused {
        //         break;
        //     }

        //     interpreter.step();
        //     println!(
        //         "pc = {}, {:?}",
        //         interpreter.instruction_counter,
        //         interpreter.group_state.puzzle_states[0].decode(
        //             &b_facelets,
        //             &PermuteCube::new_from_effect(&arch, 0, vec![(0, U512::ONE)])
        //         ),
        //     );
        // }

        assert!(matches!(
            interpreter.step_until_halt(),
            PausedState::Halt {
                message: "The modulus is",
                register: RegisterGenerator::Puzzle {
                    generator: _,
                    facelets: _
                },
                register_idx: 0,
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
