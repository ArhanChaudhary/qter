use qter_core::{Int, U, WithSpan};

#[derive(Clone, Debug)]
enum PrimitiveForOptimization {
    Add {
        amt: WithSpan<Int<U>>,
        register: RegisterReference,
    },
    Goto {
        label: WithSpan<LabelReference>,
    },
    SolvedGoto {
        label: WithSpan<LabelReference>,
        register: RegisterReference,
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
