use crate::{BlockInfoTracker, ExpansionInfo, builtin_macros::builtin_macros};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};

use internment::ArcIntern;
use pest::{Parser, error::Error, iterators::Pair};
use pest_derive::Parser;
use qter_core::{
    Int, U, WithSpan,
    architectures::{Architecture, puzzle_by_name},
    mk_error,
};

use crate::{
    Block, BlockID, BlockInfo, Code, Define, DefineValue, Label, LuaCall, Macro, MacroArgTy,
    MacroBranch, MacroCall, MacroPattern, MacroPatternComponent, ParsedSyntax, Puzzle,
    RegistersDecl, Value, lua::LuaMacros,
};

use super::Instruction;

static PRELUDE: LazyLock<ParsedSyntax> = LazyLock::new(|| {
    let prelude = ArcIntern::<str>::from(include_str!("../../qter_core/prelude.qat"));

    let mut parsed_prelude = match parse(
        &prelude,
        &|_| {
            panic!(
                "Prelude should not import files (because it's easier not to implement; message henry if you need this feature)"
            )
        },
        true,
    ) {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let builtin_macros = builtin_macros(prelude);
    parsed_prelude
        .expansion_info
        .available_macros
        .extend(builtin_macros.keys().map(|source_and_macro_name| {
            (
                source_and_macro_name.to_owned(),
                source_and_macro_name.0.to_owned(),
            )
        }));
    parsed_prelude.expansion_info.macros.extend(builtin_macros);

    parsed_prelude
});

#[derive(Parser)]
#[grammar = "./qat.pest"]
struct QatParser;

pub fn parse(
    qat: &str,
    find_import: &impl Fn(&str) -> Result<ArcIntern<str>, String>,
    is_prelude: bool,
) -> Result<ParsedSyntax, Box<Error<Rule>>> {
    let program = QatParser::parse(Rule::program, qat)?.next().unwrap();
    let qat = ArcIntern::<str>::from(qat);

    let zero_pos = program.as_span().start_pos();
    let program = program.into_inner();

    let lua_macros = match LuaMacros::new() {
        Ok(v) => v,
        Err(e) => return Err(mk_error(e.to_string(), zero_pos)),
    };

    let expansion_info = ExpansionInfo {
        block_counter: 1,
        block_info: BlockInfoTracker(HashMap::new()),
        macros: HashMap::new(),
        available_macros: HashMap::new(),
        lua_macros: HashMap::new(),
    };
    let code = Vec::new();

    let mut parsed_syntax = ParsedSyntax {
        expansion_info,
        code,
    };

    if !is_prelude {
        merge_files(
            &mut parsed_syntax,
            ArcIntern::clone(&qat),
            (*PRELUDE).to_owned(),
        );
    }

    for pair in program {
        if let Rule::EOI = pair.as_rule() {
            break;
        }

        let span = pair.as_span();
        match parse_statement(pair)? {
            Statement::Macro {
                name,
                def: macro_def,
            } => {
                let span = name.span();
                let name = ArcIntern::clone(&name);

                if parsed_syntax
                    .expansion_info
                    .macros
                    .contains_key(&(ArcIntern::clone(&qat), ArcIntern::clone(&name)))
                {
                    return Err(mk_error(
                        format!("The macro {} is already defined!", &*name),
                        span,
                    ));
                }

                parsed_syntax
                    .expansion_info
                    .macros
                    .insert((ArcIntern::clone(&qat), ArcIntern::clone(&name)), macro_def);
                parsed_syntax
                    .expansion_info
                    .available_macros
                    .insert((ArcIntern::clone(&qat), name), ArcIntern::clone(&qat));
            }
            Statement::Instruction(instruction) => {
                let span = instruction.span().to_owned();

                parsed_syntax
                    .code
                    .push(WithSpan::new((instruction.value, Some(BlockID(0))), span))
            }
            Statement::LuaBlock(code) => {
                if let Err(e) = lua_macros.add_code(code) {
                    return Err(mk_error(e.to_string(), span));
                }
            }
            Statement::Import(name) => {
                let import = match find_import(*name) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(mk_error(format!("Unable to find import: {e}"), name.span()));
                    }
                };

                let importee = parse(&import, find_import, is_prelude)?;

                merge_files(&mut parsed_syntax, ArcIntern::clone(&qat), importee);
            }
        }
    }

    parsed_syntax.expansion_info.block_info.0.insert(
        BlockID(0),
        BlockInfo {
            parent_block: None,
            child_blocks: vec![],
            registers: None,
            defines: vec![],
            labels: vec![],
        },
    );

    parsed_syntax
        .expansion_info
        .lua_macros
        .insert(qat, lua_macros);

    Ok(parsed_syntax)
}

fn merge_files(
    importer: &mut ParsedSyntax,
    importer_contents: ArcIntern<str>,
    mut importee: ParsedSyntax,
) {
    // Block numbers shouldn't be defined deeper than the root in this stage
    let block_offset = importer.expansion_info.block_counter;

    let mut max_block = 0;

    for (block_id, block_info) in importee.expansion_info.block_info.0 {
        max_block = max_block.max(block_id.0);

        importer
            .expansion_info
            .block_info
            .0
            .insert(BlockID(block_id.0 + block_offset), block_info);
    }

    importer
        .expansion_info
        .macros
        .extend(importee.expansion_info.macros);
    for (source_and_macro_name, macro_file) in importee.expansion_info.available_macros {
        // Imports should not shadow existing macros
        importer
            .expansion_info
            .available_macros
            .entry((
                ArcIntern::clone(&importer_contents),
                ArcIntern::clone(&source_and_macro_name.1),
            ))
            .or_insert_with(|| ArcIntern::clone(&macro_file));

        importer
            .expansion_info
            .available_macros
            .insert(source_and_macro_name, macro_file);
    }
    importer
        .expansion_info
        .lua_macros
        .extend(importee.expansion_info.lua_macros);

    importee.code.iter_mut().for_each(|tagged_instruction| {
        if let Some(block_id) = &mut tagged_instruction.1 {
            block_id.0 += block_offset;
        }
    });
    importer.code.extend(importee.code);
}

fn parse_registers(pair: Pair<'_, Rule>) -> Result<RegistersDecl, Box<Error<Rule>>> {
    let mut puzzles = Vec::new();

    let mut names = HashSet::new();

    for decl in pair.into_inner() {
        let puzzle = match decl.as_rule() {
            Rule::unswitchable => parse_declaration(decl)?,
            Rule::switchable => {
                let mut decls = Vec::new();

                for pair in decl.into_inner() {
                    let span = pair.as_span();

                    match parse_declaration(pair)? {
                        Puzzle::Theoretical { name: _, order: _ } => {
                            return Err(mk_error(
                                "Cannot create a switchable puzzle with a theoretical register",
                                span,
                            ));
                        }
                        Puzzle::Real { architectures } => decls.extend_from_slice(&architectures),
                    }
                }

                // TODO: Verify that the architectures are compatible with each other

                Puzzle::Real {
                    architectures: decls,
                }
            }
            rule => unreachable!("{rule:?}, {}", decl.as_str()),
        };

        let mut found_names = HashSet::new();

        match &puzzle {
            Puzzle::Theoretical { name, order: _ } => {
                found_names.insert(name.to_owned());
            }
            Puzzle::Real { architectures } => found_names.extend(
                architectures
                    .iter()
                    .flat_map(|architecture| architecture.0.iter())
                    .map(|name| name.to_owned()),
            ),
        }

        for item in found_names.into_iter() {
            if names.contains(&item) {
                return Err(mk_error("Register name is already defined", item.span()));
            } else {
                names.insert(item);
            }
        }

        puzzles.push(puzzle);
    }

    Ok(RegistersDecl {
        puzzles,
        maybe_block_id: None,
    })
}

fn parse_declaration(pair: Pair<'_, Rule>) -> Result<Puzzle, Box<Error<Rule>>> {
    let span = pair.as_span();
    let mut pairs = pair.into_inner();

    let mut regs = Vec::new();
    let mut names = HashSet::new();

    let mut arch = None;

    for pair in pairs.by_ref() {
        if let Rule::ident = pair.as_rule() {
            let name = ArcIntern::<str>::from(pair.as_str());

            if names.contains(&name) {
                return Err(mk_error("Register name is already defined", pair.as_span()));
            }
            names.insert(ArcIntern::clone(&name));

            regs.push(WithSpan::new(name, pair.as_span().into()));
        } else {
            arch = Some(pair);
            break;
        }
    }

    let arch = arch.unwrap();

    match arch.as_rule() {
        Rule::theoretical_architecture => {
            if regs.len() > 1 {
                return Err(mk_error(
                    format!(
                        "Expected one register name for a theoretical architecture, found {}",
                        regs.len()
                    ),
                    span,
                ));
            }

            let number = arch.into_inner().next().unwrap();

            Ok(Puzzle::Theoretical {
                name: regs.pop().unwrap(),
                order: WithSpan::new(
                    number.as_str().parse::<Int<U>>().unwrap(),
                    number.as_span().into(),
                ),
            })
        }
        Rule::real_architecture => {
            let arch = arch.into_inner().next().unwrap();
            let rule = arch.as_rule();
            let span = arch.as_span();
            let mut arch = arch.into_inner();

            let puzzle_name = arch.next().unwrap();
            let puzzle = match puzzle_by_name(puzzle_name.as_str()) {
                Some(v) => v,
                None => return Err(mk_error("Unknown puzzle", puzzle_name.as_span())),
            };

            let decoded_arch = match rule {
                Rule::builtin_architecture => {
                    let mut orders = Vec::new();

                    for order in arch {
                        orders.push(order.as_str().parse::<Int<U>>().unwrap());
                    }

                    match puzzle.get_preset(&orders) {
                        Some(arch) => arch,
                        None => {
                            return Err(mk_error(
                                "Could not find a builtin architecture for the given puzzle with the given orders",
                                span,
                            ));
                        }
                    }
                }
                Rule::custom_architecture => {
                    let mut algorithms = Vec::new();

                    for algorithm in arch {
                        let mut generators = Vec::new();

                        for generator in algorithm.into_inner() {
                            generators.push(ArcIntern::<str>::from(generator.as_str()));
                        }

                        algorithms.push(generators);
                    }

                    match Architecture::new(puzzle.perm_group, algorithms) {
                        Ok(v) => Arc::new(v),
                        Err(e) => {
                            return Err(mk_error(
                                format!("The generator `{e}` isn't defined for the given puzzle"),
                                span,
                            ));
                        }
                    }
                }
                rule => unreachable!("{rule:?}"),
            };

            let puzzle = Puzzle::Real {
                architectures: vec![(regs, WithSpan::new(decoded_arch, span.into()))],
            };

            Ok(puzzle)
        }
        rule => unreachable!("{rule:?}"),
    }
}

enum Statement<'a> {
    Macro {
        name: WithSpan<ArcIntern<str>>,
        def: WithSpan<Macro>,
    },
    Instruction(WithSpan<Instruction>),
    LuaBlock(&'a str),
    Import(WithSpan<&'a str>),
}

fn parse_statement(pair: Pair<'_, Rule>) -> Result<Statement<'_>, Box<Error<Rule>>> {
    let rule = pair.as_rule();

    Ok(match rule {
        Rule::r#macro => {
            let (name, macro_def) = parse_macro(pair)?;
            Statement::Macro {
                name,
                def: macro_def,
            }
        }
        Rule::instruction => Statement::Instruction(parse_instruction(pair)?),
        Rule::lua_code => Statement::LuaBlock(pair.as_str()),
        Rule::import => {
            let span = pair.as_span();
            let filename = pair.into_inner().next().unwrap();

            Statement::Import(WithSpan::new(filename.as_str(), span.into()))
        }
        _ => unreachable!("{rule:?}"),
    })
}

fn parse_instruction(pair: Pair<'_, Rule>) -> Result<WithSpan<Instruction>, Box<Error<Rule>>> {
    let pair = pair.into_inner().next().unwrap();
    let rule = pair.as_rule();
    let span = pair.as_span().into();

    Ok(WithSpan::new(
        match rule {
            Rule::label => Instruction::Label(Label {
                name: ArcIntern::<str>::from(pair.into_inner().next().unwrap().as_str()),
                maybe_block_id: None,
                available_in_blocks: None,
            }),
            Rule::code => {
                let mut pairs = pair.into_inner();

                let name = pairs.next().unwrap();
                let name =
                    WithSpan::new(ArcIntern::<str>::from(name.as_str()), name.as_span().into());

                let arguments = pairs
                    .map(|pair| parse_value(pair))
                    .collect::<Result<Vec<_>, _>>()?;

                let span = arguments
                    .iter()
                    .map(|v| v.span())
                    .fold(name.span().to_owned().after(), |acc, v| acc.merge(v));

                Instruction::Code(Code::Macro(MacroCall {
                    name,
                    arguments: WithSpan::new(arguments, span),
                }))
            }
            Rule::constant => Instruction::Constant(ArcIntern::<str>::from(
                pair.into_inner().next().unwrap().as_str(),
            )),
            Rule::lua_call => Instruction::LuaCall(parse_lua_call(pair)?),
            Rule::define => {
                let mut pairs = pair.into_inner();

                let name = pairs.next().unwrap();

                let definition = pairs.next().unwrap();

                let value = match definition.as_rule() {
                    Rule::value => DefineValue::Value(parse_value(definition)?),
                    Rule::lua_call => {
                        let span = definition.as_span();

                        DefineValue::LuaCall(WithSpan::new(
                            parse_lua_call(definition)?,
                            span.into(),
                        ))
                    }
                    rule => unreachable!("{rule:?}"),
                };

                Instruction::Define(Define {
                    name: WithSpan::new(ArcIntern::from(name.as_str()), name.as_span().into()),
                    value,
                })
            }
            Rule::registers => Instruction::Registers(parse_registers(pair)?),
            _ => unreachable!("{rule:?}"),
        },
        span,
    ))
}

fn parse_value(pair: Pair<'_, Rule>) -> Result<WithSpan<Value>, Box<Error<Rule>>> {
    let pair = pair.into_inner().next().unwrap();
    let rule = pair.as_rule();
    let span = pair.as_span().into();

    Ok(WithSpan::new(
        match rule {
            Rule::number => Value::Int(pair.as_str().parse::<Int<U>>().unwrap()),
            Rule::constant => Value::Constant(ArcIntern::from(pair.as_str())),
            Rule::ident => Value::Ident(ArcIntern::from(pair.as_str())),
            Rule::block => Value::Block(parse_block(pair)?),
            _ => unreachable!("{rule:?}"),
        },
        span,
    ))
}

fn parse_block(pair: Pair<'_, Rule>) -> Result<Block, Box<Error<Rule>>> {
    Ok(Block {
        code: pair
            .into_inner()
            .map(|pair| parse_instruction(pair).map(|instruction| instruction.map(|v| (v, None))))
            .collect::<Result<Vec<_>, _>>()?,
        maybe_id: None,
    })
}

fn parse_lua_call(pair: Pair<'_, Rule>) -> Result<LuaCall, Box<Error<Rule>>> {
    let mut pairs = pair.into_inner();

    let name = pairs.next().unwrap();

    Ok(LuaCall {
        function_name: WithSpan::new(ArcIntern::from(name.as_str()), name.as_span().into()),
        args: pairs
            .map(|pair| parse_value(pair))
            .collect::<Result<_, _>>()?,
    })
}

fn parse_macro(
    pair: Pair<'_, Rule>,
) -> Result<(WithSpan<ArcIntern<str>>, WithSpan<Macro>), Box<Error<Rule>>> {
    let span = pair.as_span();
    let mut pairs = pair.into_inner().peekable();

    let name = pairs.next().unwrap();
    let name_str = name.as_str();

    let after = pairs.peek().unwrap();

    let after = if let Rule::ident = after.as_rule() {
        Some(WithSpan::new(
            ArcIntern::from(after.as_str()),
            after.as_span().into(),
        ))
    } else {
        None
    };

    let mut branches = Vec::<WithSpan<MacroBranch>>::new();

    for branch in pairs {
        let span = branch.as_span();

        let mut pairs = branch.into_inner();

        let pattern = parse_pattern(pairs.next().unwrap())?;

        for branch in branches.iter() {
            if let Some(counterexample) = pattern.conflicts_with(name_str, &branch.pattern) {
                return Err(mk_error(
                    format!(
                        "This macro branch conflicts with the macro branch with the pattern `{}`. A counterexample matching both is `{counterexample}`.",
                        branch.pattern.span().slice()
                    ),
                    span,
                ));
            }
        }

        let body = pairs.next().unwrap();

        // TODO: Disallow macros emitting register declarations
        let body = match body.as_rule() {
            Rule::instruction => vec![parse_instruction(body)?.map(|v| (v, None))],
            Rule::block => parse_block(body)?.code,
            illegal => unreachable!("{illegal:?}"),
        };

        branches.push(WithSpan::new(
            MacroBranch {
                pattern,
                code: body,
            },
            span.into(),
        ))
    }

    Ok((
        WithSpan::new(ArcIntern::from(name.as_str()), name.as_span().into()),
        WithSpan::new(Macro::UserDefined { branches, after }, span.into()),
    ))
}

fn parse_pattern(pair: Pair<Rule>) -> Result<WithSpan<MacroPattern>, Box<Error<Rule>>> {
    let mut pattern = Vec::new();
    let span = pair.as_span();

    for pair in pair.into_inner() {
        if Rule::macro_arg != pair.as_rule() {
            break;
        }

        let span = pair.as_span();

        let mut arg_pairs = pair.into_inner();

        let first_pair = arg_pairs.next().unwrap();

        pattern.push(WithSpan::new(
            match first_pair.as_rule() {
                Rule::ident => MacroPatternComponent::Word(ArcIntern::from(first_pair.as_str())),
                Rule::constant => {
                    let name = WithSpan::new(
                        ArcIntern::from(first_pair.as_str()),
                        first_pair.as_span().into(),
                    );

                    let ty = arg_pairs.next().unwrap();

                    MacroPatternComponent::Argument {
                        name,
                        ty: WithSpan::new(
                            match ty.as_str() {
                                "block" => MacroArgTy::Block,
                                "reg" => MacroArgTy::Reg,
                                "int" => MacroArgTy::Int,
                                "ident" => MacroArgTy::Ident,
                                word => unreachable!("{word}"),
                            },
                            ty.as_span().into(),
                        ),
                    }
                }
                rule => unreachable!("{rule:?}"),
            },
            span.into(),
        ));
    }

    Ok(WithSpan::new(MacroPattern(pattern), span.into()))
}

#[cfg(test)]
mod tests {
    use internment::ArcIntern;

    use super::parse;

    #[test]
    fn bruh() {
        let code = "
            .registers {
                a, b ← 3x3 builtin (90, 90)
                (
                    c, d ← 3x3 builtin (210, 24)
                    d, e ← 3x3 builtin (30, 30, 30)
                )
                f ← theoretical 90
                g, h ← 3x3 (U, D)
            }

            .macro bruh {
                (lmao $a:reg) => add 1 $a
                (oofy $a:reg) => {
                    bruh:
                    add 1 $a
                    goto bruh
                }
            }

            .start-lua
                function bruh()
                    print(\"skibidi\")
                end
            end-lua

            bruh:
            add 1 a
            goto bruh

            lua bruh(1, 2, 3)

            .define yeet lua bruh(1, 2, 3)
            .define pog 4

            .import pog.qat
        ";

        match parse(code, &|_| Ok(ArcIntern::from("add 1 a")), false) {
            Ok(_) => {}
            Err(e) => panic!("{e}"),
        }
    }
}
