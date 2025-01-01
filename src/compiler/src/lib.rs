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
    find_import: impl Fn(&str) -> Result<ArcIntern<String>, String>,
) -> Result<Program, Box<Error<parsing::Rule>>> {
    let parsed = parse(qat, &find_import, false)?;

    let expanded = expand(parsed)?;

    strip_expanded(expanded)
}

#[derive(Clone, Debug)]
struct Label {
    name: ArcIntern<String>,
    block: Option<BlockID>,
    available_in_blocks: Option<Vec<BlockID>>,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct LabelReference {
    name: ArcIntern<String>,
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
    name: WithSpan<ArcIntern<String>>,
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
        register: RegisterReference,
    },
    Print {
        message: WithSpan<String>,
        register: RegisterReference,
    },
}

#[derive(Clone, Debug)]
enum Value {
    Int(Int<U>),
    Constant(ArcIntern<String>),
    Word(ArcIntern<String>),
    Block(Block),
}

#[derive(Clone, Debug)]
struct MacroCall {
    name: WithSpan<ArcIntern<String>>,
    arguments: WithSpan<Vec<WithSpan<Value>>>,
}

#[derive(Clone, Debug)]
enum Code {
    Primitive(Primitive),
    Macro(MacroCall),
}

#[derive(Clone, Debug)]
struct LuaCall {
    function_name: WithSpan<ArcIntern<String>>,
    args: Vec<WithSpan<Value>>,
}

#[derive(Clone, Debug)]
enum Instruction {
    Label(Label),
    Code(Code),
    Constant(ArcIntern<String>),
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
        name: WithSpan<ArcIntern<String>>,
        ty: WithSpan<PatternArgTy>,
    },
    Word(ArcIntern<String>),
}

impl PatternComponent {
    /// Returns `None` if the patterns do not conflict, otherwise returns a counterexample that would match both patterns.
    fn conflicts_with(&self, other: &PatternComponent) -> Option<ArcIntern<String>> {
        use PatternArgTy as A;
        use PatternComponent as P;

        match (self, other) {
            (P::Argument { name: _, ty: a }, P::Argument { name: _, ty: b }) => match (**a, **b) {
                (A::Int, A::Int) => Some(ArcIntern::from_ref("123")),
                (A::Reg, A::Reg)
                | (A::Ident, A::Reg)
                | (A::Reg, A::Ident)
                | (A::Ident, A::Ident) => Some(ArcIntern::from_ref("a")),
                (A::Block, A::Block) => Some(ArcIntern::from_ref("{ }")),
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
        after: Option<WithSpan<ArcIntern<String>>>,
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
    name: WithSpan<ArcIntern<String>>,
    value: DefinedValue,
}

#[derive(Clone, Debug)]
enum Cube {
    Theoretical {
        name: WithSpan<ArcIntern<String>>,
        order: WithSpan<Int<U>>,
    },
    Real {
        architectures: Vec<(
            Vec<WithSpan<ArcIntern<String>>>,
            WithSpan<Arc<Architecture>>,
        )>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BlockID(pub usize);

#[derive(Clone, Debug)]
struct RegisterDecl {
    cubes: Vec<Cube>,
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
    fn get_register(&self, reference: &RegisterReference) -> Option<(RegisterReference, &Cube)> {
        let mut from = reference.block;
        let name = reference.name.to_owned();

        loop {
            let info = self.0.get(&from)?;
            let decl = info.registers.as_ref()?;

            for cube in &decl.cubes {
                match cube {
                    Cube::Theoretical {
                        name: found_name,
                        order: _,
                    } => {
                        if *name == **found_name {
                            return Some((RegisterReference { block: from, name }, cube));
                        }
                    }
                    Cube::Real { architectures } => {
                        for (names, _) in architectures {
                            for found_name in names {
                                if *name == **found_name {
                                    return Some((RegisterReference { block: from, name }, cube));
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
    macros: HashMap<(ArcIntern<String>, ArcIntern<String>), WithSpan<Macro>>,
    /// Map each (file contents, macro name) to the file that it's in
    available_macros: HashMap<(ArcIntern<String>, ArcIntern<String>), ArcIntern<String>>,
    /// Each file has its own LuaMacros; use the file contents as the key
    lua_macros: HashMap<ArcIntern<String>, LuaMacros>,
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
