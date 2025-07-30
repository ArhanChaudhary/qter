use std::sync::Arc;

use global::do_global_optimization;
use local::do_local_optimization;
use qter_core::{
    ByPuzzleType, Int, PuzzleIdx, StateIdx, TheoreticalIdx, U, WithSpan,
    architectures::Architecture,
};

use crate::{BlockID, Label, LabelReference, RegisterReference, strip_expanded::GlobalRegs};

mod global;
mod local;

// Remove when https://doc.rust-lang.org/beta/unstable-book/language-features/deref-patterns.html is stable
#[macro_export]
macro_rules! primitive_match {
    ($pattern:pat = $val:expr) => {
        primitive_match!($pattern = $val; else { return None; });
    };

    ($pattern:pat = $val:expr; else $else:block) => {
        let OptimizingCodeComponent::Instruction(instr, _) = $val else $else;
        let $pattern = &**instr else $else;
    }
}

// NICETIES:
// - Dead code removal with real control flow analysis
// - Coalesce solved-gotos to the same label
// - Coalesce adjacent labels
// - Strength reduction of `solved-goto` after a `repeat until` or `solve` that guarantees whether or not it succeeds
// - If there's a goto immediately after a label, move the label to where the goto goes to

#[derive(Clone, Debug)]
pub enum OptimizingPrimitive {
    AddPuzzle {
        puzzle: PuzzleIdx,
        arch: Arc<Architecture>,
        // register idx, modulus, amt to add
        amts: Vec<(usize, Option<Int<U>>, WithSpan<Int<U>>)>,
    },
    AddTheoretical {
        theoretical: TheoreticalIdx,
        amt: WithSpan<Int<U>>,
    },
    Goto {
        label: WithSpan<LabelReference>,
    },
    SolvedGoto {
        label: WithSpan<LabelReference>,
        register: RegisterReference,
    },
    RepeatUntil {
        puzzle: PuzzleIdx,
        arch: Arc<Architecture>,
        amts: Vec<(usize, Option<Int<U>>, WithSpan<Int<U>>)>,
        register: RegisterReference,
    },
    Solve {
        puzzle: ByPuzzleType<'static, StateIdx>,
    },
    Input {
        message: WithSpan<String>,
        register: RegisterReference,
    },
    Halt {
        message: WithSpan<String>,
        register: Option<RegisterReference>,
    },
    Print {
        message: WithSpan<String>,
        register: Option<RegisterReference>,
    },
}

#[derive(Clone, Debug)]
pub enum OptimizingCodeComponent {
    Instruction(Box<OptimizingPrimitive>, BlockID),
    Label(Label),
}

pub fn do_optimization(
    instructions: impl Iterator<Item = WithSpan<OptimizingCodeComponent>> + Send + 'static,
    global_regs: &Arc<GlobalRegs>,
) -> Vec<WithSpan<OptimizingCodeComponent>> {
    let iter = do_local_optimization(instructions, Arc::clone(&global_regs));
    let (mut new_code, mut convergence) = do_global_optimization(iter);

    while !convergence {
        let iter = do_local_optimization(new_code.into_iter(), Arc::clone(&global_regs));
        (new_code, convergence) = do_global_optimization(iter);
    }

    new_code
}
