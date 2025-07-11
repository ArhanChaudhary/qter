use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock};

use chumsky::Parser;
use crossbeam_channel::{Receiver, Sender};
use interpreter::puzzle_states::PuzzleState;
use qter_core::{
    Int, U,
    architectures::{Algorithm, PermutationGroup, puzzle_definition},
    discrete_math::lcm_iter,
};

use crate::robot::{RobotLike, RobotLikeDyn};

use super::{InterpretationCommand, InterpretationEvent};

struct RobotHandle {
    robot: &'static mut (dyn RobotLikeDyn + Send + 'static),
    event_tx: Sender<InterpretationEvent>,
}

static ROBOT_HANDLE: OnceLock<Mutex<RobotHandle>> = OnceLock::new();

static CUBE3: LazyLock<Arc<PermutationGroup>> = LazyLock::new(|| {
    Arc::clone(
        &puzzle_definition()
            .parse(qter_core::File::from("3x3"))
            .unwrap()
            .perm_group,
    )
});

fn robot_handle() -> MutexGuard<'static, RobotHandle> {
    ROBOT_HANDLE.get().unwrap().lock().unwrap()
}

struct TrackedRobotState;

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
            .send(InterpretationEvent::CubeState(state.to_owned()));

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
                .send(InterpretationEvent::CubeState(state.to_owned()));

            state
        };

        let c = self.halt(facelets, generator)?;

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
            robot_handle().event_tx.send(InterpretationEvent::BeginHalt);
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
                    .send(InterpretationEvent::HaltCountUp(sum));
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

        Some(())
    }

    fn solve(&mut self) {
        let mut handle = robot_handle();

        handle
            .event_tx
            .send(InterpretationEvent::CubeState(CUBE3.identity()));

        handle.robot.solve();
    }
}

pub fn interpreter_loop<R: RobotLike + Send + 'static>(
    event_tx: Sender<InterpretationEvent>,
    command_rx: Receiver<InterpretationCommand>,
) {
    ROBOT_HANDLE.set(Mutex::new(RobotHandle {
        robot: Box::leak(Box::from(R::initialize(Arc::clone(&CUBE3)))),
        event_tx,
    }));
}
