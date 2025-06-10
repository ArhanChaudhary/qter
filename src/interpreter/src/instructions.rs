use qter_core::{
    ByPuzzleType, Halt, Input, Int, PerformAlgorithm, Print, RegisterGenerator,
    SeparatesByPuzzleType, SolvedGoto, U, discrete_math::lcm,
};

use crate::{
    ActionPerformed, AddAction, ExecutionState, InterpreterState, PausedState, PuzzleState,
};

pub fn do_instr<'a, Instr: PuzzleInstructionImpl, P: PuzzleState>(
    instr: &'a ByPuzzleType<'static, Instr>,
    state: &mut InterpreterState<P>,
) -> ActionPerformed<'a> {
    match instr {
        ByPuzzleType::Theoretical(instr) => Instr::perform_theoretical(instr, state),
        ByPuzzleType::Puzzle(instr) => Instr::perform_puzzle(instr, state),
    }
}

pub trait PuzzleInstructionImpl: SeparatesByPuzzleType {
    fn perform_theoretical<'a, P: PuzzleState>(
        instr: &'a Self::Theoretical<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a>;

    fn perform_puzzle<'a, P: PuzzleState>(
        instr: &'a Self::Puzzle<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a>;
}

impl PuzzleInstructionImpl for SolvedGoto {
    fn perform_theoretical<'a, P: PuzzleState>(
        instr: &'a Self::Theoretical<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        if Int::is_zero(&state.puzzle_states.theoretical_states[instr.puzzle_idx].value) {
            state.program_counter = instr.instruction_idx;

            ActionPerformed::SucceededSolvedGoto {
                facelets: ByPuzzleType::Theoretical(()),
                instruction_idx: instr.instruction_idx,
                puzzle_idx: instr.puzzle_idx,
            }
        } else {
            state.program_counter += 1;

            ActionPerformed::FailedSolvedGoto {
                facelets: ByPuzzleType::Theoretical(()),
                puzzle_idx: instr.puzzle_idx,
            }
        }
    }

    fn perform_puzzle<'a, P: PuzzleState>(
        instr: &'a Self::Puzzle<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        let puzzle = &state.puzzle_states.puzzle_states[instr.0.puzzle_idx];

        if puzzle.facelets_solved(&instr.1.0) {
            state.program_counter = instr.0.instruction_idx;

            ActionPerformed::SucceededSolvedGoto {
                facelets: ByPuzzleType::Puzzle(&instr.1),
                instruction_idx: instr.0.instruction_idx,
                puzzle_idx: instr.0.puzzle_idx,
            }
        } else {
            state.program_counter += 1;

            ActionPerformed::FailedSolvedGoto {
                facelets: ByPuzzleType::Puzzle(&instr.1),
                puzzle_idx: instr.0.puzzle_idx,
            }
        }
    }
}

fn input_impl<'a, P: PuzzleState>(
    order: Int<U>,
    input: &'a Input,
    register: ByPuzzleType<'static, RegisterGenerator>,
    state: &mut InterpreterState<P>,
) -> ActionPerformed<'a> {
    let max_input = order - Int::<U>::one();
    state.execution_state = ExecutionState::Paused(PausedState::Input {
        max_input,
        register,
        puzzle_idx: input.puzzle_idx,
    });
    state
        .messages
        .push_back(format!("{} (max input {max_input})", input.message));

    ActionPerformed::Paused
}

impl PuzzleInstructionImpl for Input {
    fn perform_theoretical<'a, P: PuzzleState>(
        instr: &'a Self::Theoretical<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        let order = state.puzzle_states.theoretical_states[instr.puzzle_idx].order;
        input_impl(order, instr, ByPuzzleType::Theoretical(()), state)
    }

    fn perform_puzzle<'a, P: PuzzleState>(
        instr: &'a Self::Puzzle<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        let order = instr
            .2
            .0
            .iter()
            .map(|facelet| instr.1.chromatic_orders_by_facelets()[*facelet])
            .fold(Int::<U>::one(), lcm);

        input_impl(
            order,
            &instr.0,
            // TODO: we should avoid the clone
            ByPuzzleType::Puzzle((instr.1.clone(), instr.2.clone())),
            state,
        )
    }
}

fn perform_halt<'a, P: PuzzleState>(
    maybe_decoded: Option<(Int<U>, (usize, ByPuzzleType<'static, RegisterGenerator>))>,
    instr: &'a Halt,
    state: &mut InterpreterState<P>,
) -> ActionPerformed<'a> {
    let full_message = if let Some((decoded, puzzle_idx_and_register)) = maybe_decoded {
        state.execution_state = ExecutionState::Paused(PausedState::Halt {
            maybe_puzzle_idx_and_register: Some(puzzle_idx_and_register),
        });

        format!("{} {decoded}", instr.message)
    } else {
        state.execution_state = ExecutionState::Paused(PausedState::Halt {
            maybe_puzzle_idx_and_register: None,
        });

        instr.message.clone()
    };
    state.messages.push_back(full_message);

    ActionPerformed::Paused
}

impl PuzzleInstructionImpl for Halt {
    fn perform_theoretical<'a, P: PuzzleState>(
        instr: &'a Self::Theoretical<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        perform_halt(
            match instr.1 {
                Some(idx) => Some((
                    state.puzzle_states.theoretical_states[idx].value,
                    (idx, ByPuzzleType::Theoretical(())),
                )),
                None => None,
            },
            &instr.0,
            state,
        )
    }

    fn perform_puzzle<'a, P: PuzzleState>(
        instr: &'a Self::Puzzle<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        perform_halt(
            match &instr.1 {
                Some((idx, algorithm, facelets)) => {
                    let puzzle = &mut state.puzzle_states.puzzle_states[*idx];
                    match puzzle.halt(&facelets.0, algorithm) {
                        Some(v) => Some((
                            v,
                            (
                                *idx,
                                ByPuzzleType::Puzzle((algorithm.to_owned(), facelets.to_owned())),
                            ),
                        )),
                        None => {
                            return state.panic("The register specified is not decodable!");
                        }
                    }
                }
                None => None,
            },
            &instr.0,
            state,
        )
    }
}

fn perform_print<'a, P: PuzzleState>(
    maybe_decoded: Option<Int<U>>,
    instr: &'a Print,
    state: &mut InterpreterState<P>,
) -> ActionPerformed<'a> {
    state.execution_state = ExecutionState::Running;

    let full_message = match maybe_decoded {
        Some(decoded) => {
            format!("{} {decoded}", instr.message)
        }
        None => instr.message.clone(),
    };
    state.messages.push_back(full_message);
    state.program_counter += 1;

    ActionPerformed::None
}

impl PuzzleInstructionImpl for Print {
    fn perform_theoretical<'a, P: PuzzleState>(
        instr: &'a Self::Theoretical<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        perform_print(
            match instr.1 {
                Some(idx) => Some(state.puzzle_states.theoretical_states[idx].value),
                None => None,
            },
            &instr.0,
            state,
        )
    }

    fn perform_puzzle<'a, P: PuzzleState>(
        instr: &'a Self::Puzzle<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        perform_print(
            match &instr.1 {
                Some((idx, algorithm, facelets)) => {
                    let puzzle = &mut state.puzzle_states.puzzle_states[*idx];
                    match puzzle.halt(&facelets.0, algorithm) {
                        Some(v) => Some(v),
                        None => {
                            return state.panic("The register specified is not decodable!");
                        }
                    }
                }
                None => None,
            },
            &instr.0,
            state,
        )
    }
}

impl PuzzleInstructionImpl for PerformAlgorithm {
    fn perform_theoretical<'a, P: PuzzleState>(
        instr: &'a Self::Theoretical<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        state.execution_state = ExecutionState::Running;

        state.puzzle_states.theoretical_states[instr.0.puzzle_idx].add_to(instr.1);

        state.program_counter += 1;

        ActionPerformed::Added(ByPuzzleType::Theoretical((
            AddAction {
                puzzle_idx: instr.0.puzzle_idx,
            },
            instr.1,
        )))
    }

    fn perform_puzzle<'a, P: PuzzleState>(
        instr: &'a Self::Puzzle<'static>,
        state: &mut InterpreterState<P>,
    ) -> ActionPerformed<'a> {
        state.execution_state = ExecutionState::Running;
        state
            .puzzle_states
            .compose_into(instr.0.puzzle_idx, &instr.1);

        state.program_counter += 1;

        ActionPerformed::Added(ByPuzzleType::Puzzle((
            AddAction {
                puzzle_idx: instr.0.puzzle_idx,
            },
            &instr.1,
        )))
    }
}
