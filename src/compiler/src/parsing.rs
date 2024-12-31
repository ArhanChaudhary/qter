use crate::builtin_macros::builtin_macros;
use std::{collections::HashMap, sync::{Arc, LazyLock}};

use internment::ArcIntern;
use pest::{
    error::{
        Error,
        ErrorVariant::{self, CustomError},
    },
    iterators::Pair,
    Parser,
};
use pest_derive::Parser;
use qter_core::{
    architectures::{puzzle_by_name, Architecture},
    Int, WithSpan, U,
};

use crate::{lua::LuaMacros, Block, BlockID, BlockInfo,  Code, Cube, Define, DefinedValue, Label, LuaCall, Macro, MacroBranch, MacroCall, ParsedSyntax, Pattern, PatternArgTy, PatternComponent, RegisterDecl, Value};

use super::Instruction;

static PRELUDE: LazyLock<ParsedSyntax> = LazyLock::new(|| {
    let str = ArcIntern::<String>::from_ref(include_str!("../../qter_core/prelude.qat"));
    
    let mut prelude = match parse(&str, &|_| panic!("Prelude should not import files (because it's easier not to implement; message henry if you need this feature)"), true) {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    prelude.macros.extend(builtin_macros(str));

    prelude
});

#[derive(Parser)]
#[grammar = "./qat.pest"]
struct QatParser;

pub fn parse(qat: &str, find_import: &impl Fn(&str) -> Result<ArcIntern<String>, String>, is_prelude: bool) -> Result<ParsedSyntax, Box<Error<Rule>>> {
    let file = ArcIntern::from_ref(qat);

    let program = QatParser::parse(Rule::program, qat)?.next().unwrap();
    let zero_pos = program.as_span().start_pos();
    let program = program.into_inner();

    let lua = match LuaMacros::new() {
        Ok(v) => v,
        Err(e) => return Err(Box::new(Error::new_from_pos(ErrorVariant::CustomError { message: e.to_string()}, zero_pos))),
    };

    let mut syntax = ParsedSyntax { block_counter: 1, block_info: HashMap::new(), macros: HashMap::new(), available_macros: HashMap::new(), lua_macros: HashMap::new(), code: Vec::new() };

    if !is_prelude {
        merge_files(&mut syntax, (*PRELUDE).to_owned());
    }

    for pair in program {
        if let Rule::EOI = pair.as_rule() {
            break;
        }
        
        let span = pair.as_span();
        match parse_statement(pair)? {
            Statement::Macro { name, macro_def } => {
                let span = name.span();
                let name = ArcIntern::clone(&name);

                if syntax.macros.contains_key(&(ArcIntern::clone(&file), ArcIntern::clone(&name))) {
                    return Err(Box::new(Error::new_from_span(ErrorVariant::CustomError { message: format!("The macro {} is already defined!", &*name) }, span.pest())));
                }
                
                syntax.macros.insert((ArcIntern::clone(&file), ArcIntern::clone(&name)), macro_def);
                syntax.available_macros.insert((ArcIntern::clone(&file), name), ArcIntern::clone(&file));
            },
            Statement::Instruction(instruction) => {
                let span = instruction.span().to_owned();

                syntax.code.push(WithSpan::new((instruction.value, BlockID(0)), span))
            },
            Statement::LuaBlock(code) => if let Err(e) = lua.add_chunk(code) {
                return Err(Box::new(Error::new_from_span(ErrorVariant::CustomError { message: e.to_string() }, span)))
            },
            Statement::Import(name) => {
                let import = match find_import(*name) {
                    Ok(v) => v,
                    Err(e) => return Err(Box::new(Error::new_from_span(ErrorVariant::CustomError { message: format!("Unable to find import: {e}") }, name.span().pest()))),
                };

                let file = parse(&import, find_import, is_prelude)?;

                merge_files(&mut syntax, file);
            },
        }
    }

    syntax.block_info.insert(BlockID(0), BlockInfo { parent: None, children: vec![], registers: None, defines: vec![] });

    syntax.lua_macros.insert(file, lua);

    Ok(syntax)
}

fn merge_files(importer: &mut ParsedSyntax, mut importee: ParsedSyntax) {
    // Block numbers shouldn't be defined deeper than the root in this stage
    let block_offset = importer.block_counter;

    let mut max_block = 0;

    for (id, block) in importee.block_info {
        max_block = max_block.max(id.0);

        importer.block_info.insert(BlockID(id.0 + block_offset), block);
    }

    importer.macros.extend(importee.macros);
    // Imports should not shadow existing macros
    for (name, macro_file) in importee.available_macros {
        importer.available_macros.entry(name).or_insert(macro_file);
    }
    importer.lua_macros.extend(importee.lua_macros);

    importee.code.iter_mut().for_each(|v| {
        v.1.0 += block_offset;
    });
    importer.code.extend(importee.code);
}

fn parse_registers(pair: Pair<'_, Rule>) -> Result<RegisterDecl, Box<Error<Rule>>> {
    let mut cubes = Vec::new();

    for decl in pair.into_inner() {
        cubes.push(match decl.as_rule() {
            Rule::unswitchable => parse_declaration(decl)?,
            Rule::switchable => {
                let mut decls = Vec::new();

                for pair in decl.into_inner() {
                    let span = pair.as_span();

                    match parse_declaration(pair)? {
                        Cube::Theoretical { name: _, order: _ } => return Err(Box::new(Error::new_from_span(
                            ErrorVariant::CustomError {
                                message:
                                    "Cannot create a switchable cube with a theoretical register"
                                        .to_owned(),
                            },
                            span,
                        ))),
                        Cube::Real { architectures } => decls.extend_from_slice(&architectures),
                    }
                }

                // TODO: Verify that the architectures are compatible with each other
                
                Cube::Real { architectures: decls }
            }
            rule => unreachable!("{rule:?}, {}", decl.as_str()),
        });
    }

    Ok(RegisterDecl { cubes, block: None })
}

fn parse_declaration(pair: Pair<'_, Rule>) -> Result<Cube, Box<Error<Rule>>> {
    let span = pair.as_span();
    let mut pairs = pair.into_inner();

    let mut regs = Vec::new();

    let mut arch = None;

    for pair in pairs.by_ref() {
        if let Rule::ident = pair.as_rule() {
            regs.push(WithSpan::new(
                ArcIntern::<String>::from_ref(pair.as_str()),
                pair.as_span().into(),
            ));
        } else {
            arch = Some(pair);
            break;
        }
    }

    let arch = arch.unwrap();

    match arch.as_rule() {
        Rule::theoretical_architecture => {
            if regs.len() > 1 {
                return Err(Box::new(Error::new_from_span(
                    CustomError {
                        message: format!(
                            "Expected one register name for a theoretical architecture, found {}",
                            regs.len()
                        ),
                    },
                    span,
                )));
            }

            let number = arch.into_inner().next().unwrap();

            Ok(Cube::Theoretical {
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
                None => {
                    return Err(Box::new(Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: "Unknown puzzle".to_string(),
                        },
                        puzzle_name.as_span(),
                    )))
                }
            };

            let decoded_arch = match rule {
                Rule::builtin_architecture => {
                    let mut orders = Vec::new();

                    for order in arch {
                        orders.push(order.as_str().parse::<Int<U>>().unwrap());
                    }

                    match puzzle.get_preset(&orders) {
                        Some(arch) => arch,
                        None => return Err(Box::new(Error::new_from_span(ErrorVariant::CustomError { message: "Could not find a builtin architecture for the given puzzle with the given orders".to_string() }, span))),
                    }
                }
                Rule::custom_architecture => {
                    let mut algorithms = Vec::new();

                    for algorithm in arch {
                        let mut generators = Vec::new();

                        for generator in algorithm.into_inner() {
                            generators.push(ArcIntern::<String>::from_ref(generator.as_str()));
                        }

                        algorithms.push(generators);
                    }

                    match Architecture::new(puzzle.group, algorithms) {
                        Ok(v) => Arc::new(v),
                        Err(e) => {
                            return Err(Box::new(Error::new_from_span(
                                ErrorVariant::CustomError {
                                    message: format!(
                                        "The generator `{e}` isn't defined for the given puzzle"
                                    ),
                                },
                                span,
                            )));
                        }
                    }
                }
                rule => unreachable!("{rule:?}"),
            };

            let cube = Cube::Real {
                architectures: vec![(regs, WithSpan::new(decoded_arch, span.into()))],
            };

            Ok(cube)
        }
        rule => unreachable!("{rule:?}"),
    }
}

enum Statement<'a> {
    Macro { name: WithSpan<ArcIntern<String>>, macro_def: WithSpan<Macro> },
    Instruction(WithSpan<Instruction>),
    LuaBlock(&'a str),
    Import(WithSpan<&'a str>),
}

fn parse_statement(pair: Pair<'_, Rule>) -> Result<Statement<'_>, Box<Error<Rule>>> {
    let rule = pair.as_rule();

    Ok(match rule {
        Rule::r#macro => {
            let (name, macro_def) = parse_macro(pair)?;
            Statement::Macro { name, macro_def }
        },
        Rule::instruction => Statement::Instruction(parse_instruction(pair)?),
        Rule::lua_code => {
            Statement::LuaBlock(pair.as_str())
        },
        Rule::import => {
            let span = pair.as_span();
            let filename = pair.into_inner().next().unwrap();

            Statement::Import(WithSpan::new(filename.as_str(), span.into()))
        },
        _ => unreachable!("{rule:?}"),
    })
}

fn parse_instruction(pair: Pair<'_, Rule>) -> Result<WithSpan<Instruction>, Box<Error<Rule>>> {
    let pair = pair.into_inner().next().unwrap();
    let rule = pair.as_rule();
    let span = pair.as_span().into();

    Ok(WithSpan::new(match rule {
        Rule::label => Instruction::Label(Label { name: ArcIntern::<String>::from_ref(pair.into_inner().next().unwrap().as_str()), block: None, available_in_blocks: None } ),
        Rule::code => {
            let mut pairs = pair.into_inner();

            let name = pairs.next().unwrap();
            let name = WithSpan::new(ArcIntern::<String>::from_ref(name.as_str()), name.as_span().into());

            let arguments = pairs.map(|v| parse_value(v)).collect::<Result<Vec<_>, _>>()?;
            
            Instruction::Code(Code::Macro(MacroCall { name, arguments }))
        },
        Rule::constant => Instruction::Constant(ArcIntern::<String>::from_ref(pair.into_inner().next().unwrap().as_str())),
        Rule::lua_call => {
            Instruction::LuaCall(parse_lua_call(pair)?)
        },
        Rule::define=> {
            let mut pairs = pair.into_inner();

            let name = pairs.next().unwrap();

            let definition = pairs.next().unwrap();

            let value = match definition.as_rule() {
                Rule::value => DefinedValue::Value(parse_value(definition)?),
                Rule::lua_call => {
                    let span = definition.as_span();

                    DefinedValue::LuaCall(WithSpan::new(parse_lua_call(definition)?, span.into()))
                },
                rule => unreachable!("{rule:?}"),
            };

            Instruction::Define(Define { name: WithSpan::new(ArcIntern::from_ref(name.as_str()), name.as_span().into()), value })
        },
        Rule::registers=> Instruction::Registers(parse_registers(pair)?),
        _ => unreachable!("{rule:?}")
    }, span))
}

fn parse_value(pair: Pair<'_, Rule>) -> Result<WithSpan<Value>, Box<Error<Rule>>> {
    let pair = pair.into_inner().next().unwrap();
    let rule = pair.as_rule();
    let span = pair.as_span().into();
    
    Ok(WithSpan::new(match rule {
        Rule::number => Value::Int(pair.as_str().parse::<Int<U>>().unwrap()),
        Rule::constant=> Value::Constant(ArcIntern::from_ref(pair.as_str())),
        Rule::ident => Value::Word(ArcIntern::from_ref(pair.as_str())),
        Rule::block=> Value::Block ( parse_block(pair)? ),
        _ => unreachable!("{rule:?}")
    }, span))
}

fn parse_block(pair: Pair<'_, Rule>) -> Result<Block, Box<Error<Rule>>> {
    Ok(Block { code: 
    pair.into_inner().map(|v| parse_instruction(v)).collect::<Result<Vec<_>, _>>()?
        , block: None })
}

fn parse_lua_call(pair: Pair<'_, Rule>) -> Result<LuaCall, Box<Error<Rule>>> {
            let mut pairs = pair.into_inner();

            let name = pairs.next().unwrap();
            
            Ok(LuaCall { function_name: WithSpan::new(ArcIntern::from_ref(name.as_str()), name.as_span().into()), args: pairs.map(|v| parse_value(v)).collect::<Result<_, _>>()? })
    
}

fn parse_macro(pair: Pair<'_, Rule>) -> Result<(WithSpan<ArcIntern<String>>, WithSpan<Macro>), Box<Error<Rule>>> {
    let span = pair.as_span();
    let mut pairs = pair.into_inner().peekable();

    let name = pairs.next().unwrap();
    let name_str = name.as_str();

    let after = pairs.peek().unwrap();

    let after = if let Rule::ident = after.as_rule() {
        Some(WithSpan::new(ArcIntern::from_ref(after.as_str()), after.as_span().into()))
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
                return Err(Box::new(Error::new_from_span(ErrorVariant::CustomError { message: format!("This macro branch conflicts with the macro branch with the pattern `{}`. A counterexample matching both is `{counterexample}`.", branch.pattern.span().slice()) }, span)))
            }
        }

        let body = pairs.next().unwrap();

        // TODO: Disallow macros emitting register declarations
        let body = match body.as_rule() {
            Rule::instruction => vec![parse_instruction(body)?],
            Rule::block => parse_block(body)?.code,
            rule => unreachable!("{rule:?}"),
        };

        branches.push(WithSpan::new(MacroBranch { pattern, code: body }, span.into()))
    }

    Ok((WithSpan::new(ArcIntern::from_ref(name.as_str()), name.as_span().into()), WithSpan::new(Macro::Splice { branches, after }, span.into())))
}

fn parse_pattern(pair: Pair<Rule>) -> Result<WithSpan<Pattern>, Box<Error<Rule>>> {
        let mut pattern = Vec::new();
        let span = pair.as_span();

        for pair in pair.into_inner() {
            if Rule::macro_arg != pair.as_rule() {
                break
            }

            let span = pair.as_span();

           let mut arg_pairs = pair.into_inner();

            let first_pair = arg_pairs.next().unwrap();

            pattern.push(WithSpan::new(match first_pair.as_rule() {
                Rule::ident => PatternComponent::Word(ArcIntern::from_ref(first_pair.as_str())),
                Rule::constant => {
                    let name = WithSpan::new(ArcIntern::from_ref(first_pair.as_str()), first_pair.as_span().into());

                    let ty = arg_pairs.next().unwrap();

                    PatternComponent::Argument { name, ty: WithSpan::new(match ty.as_str() {
                        "block" => PatternArgTy::Block,
                        "reg" => PatternArgTy::Reg,
                        "int" => PatternArgTy::Int,
                        "ident" => PatternArgTy::Ident,
                        word => unreachable!("{word}"),
                    }, ty.as_span().into()) }
                },
                rule => unreachable!("{rule:?}")
            }, span.into()));
        }

        Ok(WithSpan::new(Pattern(pattern), span.into()))
    
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

        match parse(code, &|_| Ok(ArcIntern::from_ref("add 1 a")), false) {
            Ok(_) => {}
            Err(e) => panic!("{e}"),
        }
    }
}
