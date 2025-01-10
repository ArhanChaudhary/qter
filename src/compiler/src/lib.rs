use std::{collections::HashMap, sync::Arc};

use internment::ArcIntern;
use lua::LuaMacros;
use parsing::{parse, Rule};
use pest::error::Error;
use qter_core::{architectures::Architecture, Int, Program, WithSpan, U};
use strip_expanded::strip_expanded;

use crate::macro_expansion::expand;

mod builtin_macros;
mod lua;
mod macro_expansion;
mod parsing;
mod strip_expanded;

pub fn compile(
    qat: &str,
    find_import: impl Fn(&str) -> Result<ArcIntern<str>, String>,
) -> Result<Program, Box<Error<parsing::Rule>>> {
    let parsed = parse(qat, &find_import, false)?;

    let expanded = expand(parsed)?;

    strip_expanded(expanded)
}

#[derive(Clone, Debug)]
struct Label {
    name: ArcIntern<str>,
    block: Option<BlockID>,
    available_in_blocks: Option<Vec<BlockID>>,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct LabelReference {
    name: ArcIntern<str>,
    block: BlockID,
}

#[derive(Clone, Debug)]
struct Block {
    code: Vec<WithSpan<(Instruction, Option<BlockID>)>>,
    block: Option<BlockID>,
}

#[derive(Clone, Debug)]
struct RegisterReference {
    block: BlockID,
    name: WithSpan<ArcIntern<str>>,
}

#[derive(Clone, Debug)]
enum Primitive {
    Add {
        amt: WithSpan<Int<U>>,
        register: RegisterReference,
    },
    Goto {
        label: WithSpan<LabelReference>,
    },
    SolvedGoto {
        register: RegisterReference,
        label: WithSpan<LabelReference>,
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
enum Value {
    Int(Int<U>),
    Constant(ArcIntern<str>),
    Word(ArcIntern<str>),
    Block(Block),
}

#[derive(Clone, Debug)]
struct MacroCall {
    name: WithSpan<ArcIntern<str>>,
    arguments: WithSpan<Vec<WithSpan<Value>>>,
}

#[derive(Clone, Debug)]
enum Code {
    Primitive(Primitive),
    Macro(MacroCall),
}

#[derive(Clone, Debug)]
struct LuaCall {
    function_name: WithSpan<ArcIntern<str>>,
    args: Vec<WithSpan<Value>>,
}

#[derive(Clone, Debug)]
enum Instruction {
    Label(Label),
    Code(Code),
    Constant(ArcIntern<str>),
    LuaCall(LuaCall),
    Define(Define),
    Registers(RegisterDecl),
}

#[derive(Clone, Copy, Debug)]
enum PatternArgTy {
    Int,
    Reg,
    Block,
    Ident,
}

#[derive(Clone, Debug)]
enum PatternComponent {
    Argument {
        name: WithSpan<ArcIntern<str>>,
        ty: WithSpan<PatternArgTy>,
    },
    Word(ArcIntern<str>),
}

impl PatternComponent {
    /// Returns `None` if the patterns do not conflict, otherwise returns a counterexample that would match both patterns.
    fn conflicts_with(&self, other: &PatternComponent) -> Option<ArcIntern<str>> {
        use PatternArgTy as A;
        use PatternComponent as P;

        match (self, other) {
            (P::Argument { name: _, ty: a }, P::Argument { name: _, ty: b }) => match (**a, **b) {
                (A::Int, A::Int) => Some(ArcIntern::from("123")),
                (A::Reg, A::Reg)
                | (A::Ident, A::Reg)
                | (A::Reg, A::Ident)
                | (A::Ident, A::Ident) => Some(ArcIntern::from("a")),
                (A::Block, A::Block) => Some(ArcIntern::from("{ }")),
                _ => None,
            },
            (P::Argument { name: _, ty }, P::Word(word))
            | (P::Word(word), P::Argument { name: _, ty }) => match **ty {
                A::Ident | A::Reg => Some(ArcIntern::clone(word)),
                _ => None,
            },
            (P::Word(a), P::Word(b)) => (a == b).then(|| ArcIntern::clone(a)),
        }
    }
}

#[derive(Clone, Debug)]
struct Pattern(Vec<WithSpan<PatternComponent>>);

impl Pattern {
    /// Returns `None` if the patterns do not conflict, otherwise returns a counterexample that would match both patterns.
    pub fn conflicts_with(&self, macro_name: &str, other: &Pattern) -> Option<String> {
        if self.0.len() != other.0.len() {
            return None;
        }

        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| a.conflicts_with(b))
            .try_fold(String::new(), |mut a, v| {
                let v = v?;

                a.push(' ');
                a.push_str(&v);
                Some(a)
            })
            .map(|e| format!("{macro_name}{e}"))
    }
}

#[derive(Clone, Debug)]
enum ValueOrReg {
    Value(Value),
    Register(RegisterReference),
}

#[derive(Clone, Debug)]
struct MacroBranch {
    pattern: WithSpan<Pattern>,
    code: Vec<WithSpan<(Instruction, Option<BlockID>)>>,
}

#[derive(Clone, Debug)]
enum Macro {
    Splice {
        branches: Vec<WithSpan<MacroBranch>>,
        after: Option<WithSpan<ArcIntern<str>>>,
    },
    Builtin(
        fn(
            &ExpansionInfo,
            WithSpan<Vec<WithSpan<Value>>>,
            BlockID,
        ) -> Result<Vec<Instruction>, Box<Error<Rule>>>,
    ),
}

#[derive(Clone, Debug)]
enum DefinedValue {
    Value(WithSpan<Value>),
    LuaCall(WithSpan<LuaCall>),
}

#[derive(Clone, Debug)]
struct Define {
    name: WithSpan<ArcIntern<str>>,
    value: DefinedValue,
}

#[derive(Clone, Debug)]
enum Puzzle {
    Theoretical {
        name: WithSpan<ArcIntern<str>>,
        order: WithSpan<Int<U>>,
    },
    Real {
        architectures: Vec<(Vec<WithSpan<ArcIntern<str>>>, WithSpan<Arc<Architecture>>)>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BlockID(pub usize);

#[derive(Clone, Debug)]
struct RegisterDecl {
    puzzles: Vec<Puzzle>,
    block: Option<BlockID>,
}

#[derive(Debug, Clone)]
struct BlockInfo {
    parent: Option<BlockID>,
    children: Vec<BlockID>,
    registers: Option<RegisterDecl>,
    defines: Vec<Define>,
    labels: Vec<Label>,
}

#[derive(Debug, Clone)]
struct BlockInfoTracker(HashMap<BlockID, BlockInfo>);

impl BlockInfoTracker {
    fn get_register(&self, reference: &RegisterReference) -> Option<(RegisterReference, &Puzzle)> {
        let mut from = reference.block;
        let name = reference.name.to_owned();

        loop {
            let info = self.0.get(&from)?;
            let decl = info.registers.as_ref()?;

            for puzzle in &decl.puzzles {
                match puzzle {
                    Puzzle::Theoretical {
                        name: found_name,
                        order: _,
                    } => {
                        if *name == **found_name {
                            return Some((RegisterReference { block: from, name }, puzzle));
                        }
                    }
                    Puzzle::Real { architectures } => {
                        for (names, _) in architectures {
                            for found_name in names {
                                if *name == **found_name {
                                    return Some((RegisterReference { block: from, name }, puzzle));
                                }
                            }
                        }
                    }
                }
            }

            from = info.parent?;
        }
    }

    fn label_scope(&self, reference: &LabelReference) -> Option<LabelReference> {
        let mut current = reference.block;

        loop {
            let info = self.0.get(&current)?;

            for label in &info.labels {
                if label.name == reference.name {
                    if let Some(available_in) = &label.available_in_blocks {
                        if available_in.contains(&reference.block) {
                            return Some(LabelReference {
                                name: ArcIntern::clone(&reference.name),
                                block: current,
                            });
                        }
                    } else {
                        return Some(LabelReference {
                            name: ArcIntern::clone(&reference.name),
                            block: current,
                        });
                    };
                }
            }

            current = info.parent?;
        }
    }
}

#[derive(Clone, Debug)]
struct ExpansionInfo {
    // Each block gets an ID and `block_parent` maps a block ID to it's parent
    // The global scope is block zero and if the block/label hasn't been expanded its ID is None
    block_counter: usize,
    block_info: BlockInfoTracker,
    /// Map (file contents, macro name) to a macro
    macros: HashMap<(ArcIntern<str>, ArcIntern<str>), WithSpan<Macro>>,
    /// Map each (file contents, macro name) to the file that it's in
    available_macros: HashMap<(ArcIntern<str>, ArcIntern<str>), ArcIntern<str>>,
    /// Each file has its own LuaMacros; use the file contents as the key
    lua_macros: HashMap<ArcIntern<str>, LuaMacros>,
}

#[derive(Clone, Debug)]
struct ParsedSyntax {
    expansion_info: ExpansionInfo,
    code: Vec<WithSpan<(Instruction, Option<BlockID>)>>,
}

#[derive(Clone, Debug)]
enum ExpandedCode {
    Instruction(Primitive, BlockID),
    Label(Label),
}

#[derive(Clone, Debug)]
struct Expanded {
    block_info: BlockInfoTracker,
    code: Vec<WithSpan<ExpandedCode>>,
}
