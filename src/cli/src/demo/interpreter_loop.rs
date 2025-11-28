use std::{
    sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock},
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};
use interpreter::{
    ActionPerformed, ExecutionState, Interpreter, PausedState,
    puzzle_states::{PuzzleState, RobotLike, RobotLikeDyn},
};
use qter_core::{
    Facelets, Int, U,
    architectures::{Algorithm, PermutationGroup, PuzzleDefinition, mk_puzzle_definition},
    discrete_math::lcm_iter,
};

use crate::demo::PROGRAMS;

use super::{InterpretationCommand, interpreter_plugin::InterpretationEvent};

struct RobotHandle {
    robot: &'static mut (dyn RobotLikeDyn + Send + 'static),
    event_tx: Sender<InterpretationEvent>,
}

static ROBOT_HANDLE: OnceLock<Mutex<RobotHandle>> = OnceLock::new();

pub static CUBE3_DEF: LazyLock<Arc<PuzzleDefinition>> =
    LazyLock::new(|| mk_puzzle_definition("3x3").unwrap());

pub static CUBE3: LazyLock<Arc<PermutationGroup>> =
    LazyLock::new(|| Arc::clone(&CUBE3_DEF.perm_group));

fn robot_handle() -> MutexGuard<'static, RobotHandle> {
    ROBOT_HANDLE.get().unwrap().lock().unwrap()
}

struct TrackedRobotState;

impl TrackedRobotState {
    fn halt_quiet(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<Int<U>> {
        let mut generator = generator.to_owned();
        generator.exponentiate(-Int::<U>::one());

        let mut sum = Int::<U>::zero();

        let chromatic_orders = generator.chromatic_orders_by_facelets();
        let order = lcm_iter(facelets.iter().map(|&i| chromatic_orders[i]));

        while !self.facelets_solved(facelets) {
            sum += Int::<U>::one();

            if sum >= order {
                eprintln!(
                    "Decoding failure! Performed as many cycles as the size of the register."
                );
                return None;
            }

            self.compose_into(&generator);
        }

        Some(sum)
    }
}

impl PuzzleState for TrackedRobotState {
    fn initialize(_: Arc<PermutationGroup>) -> Self {
        robot_handle().robot.solve();

        TrackedRobotState
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        robot_handle().robot.compose_into(alg);
    }

    fn facelets_solved(&self, facelets: &[usize]) -> bool {
        let handle = robot_handle();
        let state = handle.robot.take_picture();

        handle
            .event_tx
            .send(InterpretationEvent::CubeState(state.to_owned()))
            .unwrap();

        for &facelet in facelets {
            let maps_to = state.mapping()[facelet];
            if CUBE3.facelet_colors()[maps_to] != CUBE3.facelet_colors()[facelet] {
                return false;
            }
        }

        true
    }

    fn print(
        &mut self,
        facelets: &[usize],
        generator: &Algorithm,
    ) -> Option<qter_core::Int<qter_core::U>> {
        let before = {
            let handle = robot_handle();

            let state = handle.robot.take_picture().to_owned();

            handle
                .event_tx
                .send(InterpretationEvent::CubeState(state.clone()))
                .unwrap();

            state
        };

        let c = self.halt_quiet(facelets, generator)?;

        let mut exponentiated = generator.to_owned();
        exponentiated.exponentiate(c.into());

        self.compose_into(&exponentiated);

        let handle = robot_handle();

        if &before != handle.robot.take_picture() {
            eprintln!("Printing did not return the cube to the original state!");
            return None;
        }
        Some(c)
    }

    fn halt(
        &mut self,
        facelets: &[usize],
        generator: &Algorithm,
    ) -> Option<qter_core::Int<qter_core::U>> {
        {
            robot_handle()
                .event_tx
                .send(InterpretationEvent::BeginHalt {
                    facelets: Facelets(facelets.to_owned()),
                })
                .unwrap();
        }

        let mut generator = generator.to_owned();
        generator.exponentiate(-Int::<U>::one());

        let mut sum = Int::<U>::zero();

        let chromatic_orders = generator.chromatic_orders_by_facelets();
        let order = lcm_iter(facelets.iter().map(|&i| chromatic_orders[i]));

        while !self.facelets_solved(facelets) {
            sum += Int::<U>::one();

            {
                robot_handle()
                    .event_tx
                    .send(InterpretationEvent::HaltCountUp(sum))
                    .unwrap();
            }

            if sum >= order {
                eprintln!(
                    "Decoding failure! Performed as many cycles as the size of the register."
                );
                return None;
            }

            self.compose_into(&generator);
        }

        Some(sum)
    }

    fn repeat_until(&mut self, facelets: &[usize], generator: &Algorithm) -> Option<()> {
        // repeat_until has the same behavior as halt
        self.halt_quiet(facelets, generator).map(|_| ())
    }

    fn solve(&mut self) {
        let mut handle = robot_handle();

        handle
            .event_tx
            .send(InterpretationEvent::CubeState(CUBE3.identity()))
            .unwrap();

        handle.robot.solve();
    }
}

pub fn interpreter_loop<R: RobotLike + Send + 'static>(
    event_tx: Sender<InterpretationEvent>,
    command_rx: Receiver<InterpretationCommand>,
) {
    if ROBOT_HANDLE
        .set(Mutex::new(RobotHandle {
            robot: Box::leak(Box::from(R::initialize(Arc::clone(&CUBE3)))),
            event_tx,
        }))
        .is_err()
    {
        panic!("Cannot create multiple interpreter threads")
    }

    let mut maybe_interpreter = None;

    for command in command_rx {
        use InterpretationCommand as C;

        let mut halted = false;

        match command {
            C::Execute(name) => {
                maybe_interpreter = Some(Interpreter::<TrackedRobotState>::new(Arc::clone(
                    &PROGRAMS.get(&name).unwrap().program,
                )));

                robot_handle()
                    .event_tx
                    .send(InterpretationEvent::BeganProgram(name))
                    .unwrap();
            }
            C::Step => {
                let Some(interpreter) = &mut maybe_interpreter else {
                    eprintln!("Cannot step while the interpreter is closed");
                    continue;
                };

                thread::sleep(Duration::from_millis(250));

                use ActionPerformed as A;

                let instr = interpreter.state().program_counter();

                robot_handle()
                    .event_tx
                    .send(InterpretationEvent::ExecutingInstruction { which_one: instr })
                    .unwrap();

                match interpreter.step() {
                    A::Goto { instruction_idx: _ }
                    | A::Added(_)
                    | A::Solved(_)
                    | A::RepeatedUntil {
                        puzzle_idx: _,
                        facelets: _,
                        alg: _,
                    }
                    | A::None => {}
                    A::Paused => match interpreter.state().execution_state() {
                        ExecutionState::Running => unreachable!(),
                        ExecutionState::Paused(paused_state) => match paused_state {
                            PausedState::Halt {
                                maybe_puzzle_idx_and_register: _,
                            } => {
                                robot_handle()
                                    .event_tx
                                    .send(InterpretationEvent::FinishedProgram)
                                    .unwrap();
                                halted = true;
                            }
                            PausedState::Input { max_input, data: _ } => {
                                robot_handle()
                                    .event_tx
                                    .send(InterpretationEvent::Input(*max_input))
                                    .unwrap();
                            }
                            PausedState::Panicked => unreachable!(),
                        },
                    },
                    A::FailedSolvedGoto(by_puzzle_type) => match by_puzzle_type {
                        qter_core::ByPuzzleType::Theoretical(_) => unreachable!(),
                        qter_core::ByPuzzleType::Puzzle((_, facelets)) => robot_handle()
                            .event_tx
                            .send(InterpretationEvent::SolvedGoto {
                                facelets: facelets.clone(),
                            })
                            .unwrap(),
                    },
                    A::SucceededSolvedGoto(by_puzzle_type) => match by_puzzle_type {
                        qter_core::ByPuzzleType::Theoretical(_) => unreachable!(),
                        qter_core::ByPuzzleType::Puzzle((_, _, facelets)) => robot_handle()
                            .event_tx
                            .send(InterpretationEvent::SolvedGoto {
                                facelets: facelets.clone(),
                            })
                            .unwrap(),
                    },
                    A::Panicked => {
                        eprintln!("The interpreter panicked!");
                        halted = true;
                        robot_handle()
                            .event_tx
                            .send(InterpretationEvent::FinishedProgram)
                            .unwrap();
                    }
                }

                while let Some(interpreter_message) = interpreter.state_mut().messages().pop_front()
                {
                    robot_handle()
                        .event_tx
                        .send(InterpretationEvent::Message(interpreter_message))
                        .unwrap();
                }

                robot_handle()
                    .event_tx
                    .send(InterpretationEvent::DoneExecuting)
                    .unwrap();

                if halted {
                    maybe_interpreter = None;
                }
            }
            C::GiveInput(int) => {
                let Some(interpreter) = &mut maybe_interpreter else {
                    eprintln!("Cannot give input when there is no interpreter set");
                    continue;
                };

                if let ExecutionState::Paused(PausedState::Input {
                    max_input: _,
                    data: _,
                }) = interpreter.state().execution_state()
                {
                    if let Err(msg) = interpreter.give_input(int) {
                        robot_handle()
                            .event_tx
                            .send(InterpretationEvent::Message(msg))
                            .unwrap();
                    } else {
                        robot_handle()
                            .event_tx
                            .send(InterpretationEvent::GaveInput)
                            .unwrap();
                    }
                } else {
                    eprintln!("Cannot give input when there is no input instruction");
                }
            }
            C::Solve => {
                maybe_interpreter = None;

                TrackedRobotState.solve();
            }
        }
    }
}
