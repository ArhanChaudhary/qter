#![feature(gen_blocks)]

use std::sync::{Arc, LazyLock};

use interpreter::puzzle_states::RobotLike;
use qter_core::architectures::{Algorithm, Permutation, PermutationGroup, mk_puzzle_definition};

use crate::{hardware::RobotHandle, rob_twophase::solve_rob_twophase};

pub mod hardware;
pub mod rob_twophase;

pub static CUBE3: LazyLock<Arc<PermutationGroup>> =
    LazyLock::new(|| Arc::clone(&mk_puzzle_definition("3x3").unwrap().perm_group));

pub struct QterRobot {
    state: Permutation,
    handle: RobotHandle,
}

impl RobotLike for QterRobot {
    type InitializationArgs = RobotHandle;

    fn initialize(group: Arc<PermutationGroup>, handle: RobotHandle) -> Self {
        assert_eq!(group.definition().slice(), "3x3");
        
        QterRobot {
            handle,
            state: CUBE3.identity(),
        }
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        self.state.compose_into(alg.permutation());

        self.handle.queue_move_seq(alg);
    }

    fn take_picture(&mut self) -> &Permutation {
        self.handle.await_moves();
        &self.state
    }

    fn solve(&mut self) {
        let alg = solve_rob_twophase(self.take_picture().clone()).unwrap();

        self.compose_into(&alg);
    }
}
