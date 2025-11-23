use std::sync::{Arc, LazyLock};

use qter_core::architectures::{PermutationGroup, mk_puzzle_definition};

pub mod hardware;
mod rob_twophase;

pub static CUBE3: LazyLock<Arc<PermutationGroup>> = LazyLock::new(|| {
    Arc::clone(
        &mk_puzzle_definition("3x3")
            .unwrap()
            .perm_group,
    )
});
