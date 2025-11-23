use std::{path::PathBuf, sync::{Arc, LazyLock}};

use interpreter::puzzle_states::RobotLike;
use qter_core::architectures::{Algorithm, Permutation, PermutationGroup, mk_puzzle_definition};

use crate::{hardware::{RobotConfig, init, run_move_seq}, rob_twophase::solve_rob_twophase};

pub mod hardware;
mod rob_twophase;

pub static CUBE3: LazyLock<Arc<PermutationGroup>> = LazyLock::new(|| {
    Arc::clone(
        &mk_puzzle_definition("3x3")
            .unwrap()
            .perm_group,
    )
});

pub struct QterRobot {
    state: Permutation,
    config: RobotConfig,
}

impl RobotLike for QterRobot {
    fn initialize(_: Arc<PermutationGroup>) -> Self {
        // TODO: Better way of getting the config. Maybe use `include_str!`?
        let config = init(&PathBuf::from("robot_config.toml"));
        
        QterRobot { config, state: CUBE3.identity() }
    }

    fn compose_into(&mut self, alg: &Algorithm) {
        self.state.compose_into(alg.permutation());

        run_move_seq(&self.config, alg);
    }

    fn take_picture(&self) -> &Permutation {
        &self.state
    }

    fn solve(&mut self) {
        let alg = solve_rob_twophase(self.take_picture().clone()).unwrap();

        self.compose_into(&alg);
    }
}
