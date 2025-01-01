use std::{cell::OnceCell, mem};

use internment::ArcIntern;
use itertools::Itertools;
use pest::error::{Error, ErrorVariant};
use qter_core::WithSpan;

use crate::{BlockID, Code, Expanded, ExpandedCode, ExpansionInfo, Instruction, ParsedSyntax};

use super::parsing::Rule;

pub fn expand(mut parsed: ParsedSyntax) -> Result<Expanded, Box<Error<Rule>>> {
    // TODO: Logic of `after`
    while expand_block(BlockID(0), &mut parsed.expansion_info, &mut parsed.code)? {}

    Ok(Expanded {
        block_info: parsed.expansion_info.block_info,
        code: parsed
            .code
            .into_iter()
            .map(|v| {
                let span = v.span().to_owned();
                let (instruction, id) = v.into_inner();

                let expanded = match instruction {
                    Instruction::Label(label) => ExpandedCode::Label(label),
                    Instruction::Code(Code::Primitive(primitive)) => {
                        ExpandedCode::Instruction(primitive, id.unwrap())
                    }
                    i => unreachable!("{i:?}"),
                };

                WithSpan::new(expanded, span)
            })
            .collect_vec(),
    })
}

/// Returns whether any changes were made
fn expand_block(
    id: BlockID,
    info: &mut ExpansionInfo,
    code: &mut Vec<WithSpan<(Instruction, Option<BlockID>)>>,
) -> Result<bool, Box<Error<Rule>>> {
    // Will be set if anything is ever changed
    let changed = OnceCell::<()>::new();

    *code = mem::take(code)
        .into_iter()
        .map(|mut instruction| {
            if instruction.1.is_none() {
                instruction.1 = Some(id);
                let _ = changed.set(());
            }

            instruction
        })
        .flat_map(|v| {
            let id = v.1.unwrap();

            let block_info = info.block_info.get_mut(&id).unwrap();

            let span = v.span().to_owned();
            let instruction = v.into_inner();

            match instruction.0 {
                Instruction::Label(mut label) => {
                    if label.block.is_none() {
                        label.block = Some(id);
                        let _ = changed.set(());
                    }

                    block_info.labels.push(label.to_owned());

                    vec![Ok(WithSpan::new(
                        (Instruction::Label(label), instruction.1),
                        span,
                    ))]
                }
                Instruction::Define(define) => {
                    for found_define in &block_info.defines {
                        if *found_define.name == *define.name {
                            return vec![Err(Box::new(Error::new_from_span(
                                ErrorVariant::CustomError {
                                    message: "Cannot shadow a `.define` in the same scope!"
                                        .to_string(),
                                },
                                define.name.span().pest(),
                            )))];
                        }
                    }

                    block_info.defines.push(define);
                    let _ = changed.set(());

                    vec![]
                }
                Instruction::Registers(decl) => match block_info.registers {
                    Some(_) => {
                        vec![Err(Box::new(Error::new_from_span(
                            ErrorVariant::CustomError {
                                message: "Cannot have multiple register declarations in the same scope!".to_string(),
                            },
                            span.pest(),
                        )))]
                    }

                    None => {
                        block_info.registers = Some(decl);
                        let _ = changed.set(());
                        vec![]
                    }
                },
                Instruction::Code(code) => {
                    match expand_code(instruction.1.unwrap(), info, code, &changed) {
                        Ok(v) => v
                            .into_iter()
                            .map(|v| Ok(WithSpan::new(v, span.to_owned())))
                            .collect_vec(),
                        Err(e) => vec![Err(e)],
                    }
                }
                Instruction::Constant(_) => todo!(),
                Instruction::LuaCall(_) => todo!(),
            }
        })
        .collect::<Result<_, _>>()?;

    Ok(changed.get().is_some())
}

fn expand_code(
    id: BlockID,
    info: &mut ExpansionInfo,
    code: Code,
    changed: &OnceCell<()>,
) -> Result<Vec<(Instruction, Option<BlockID>)>, Box<Error<Rule>>> {
    let macro_call = match code {
        Code::Primitive(v) => return Ok(vec![(Instruction::Code(Code::Primitive(v)), Some(id))]),
        Code::Macro(v) => v,
    };

    let _ = changed.set(());

    let macro_access = match info.available_macros.get(&(
        macro_call.name.span().source(),
        ArcIntern::clone(&*macro_call.name),
    )) {
        Some(v) => v,
        None => {
            return Err(Box::new(Error::new_from_span(
                ErrorVariant::CustomError {
                    message: "Macro was not found in this scope".to_string(),
                },
                macro_call.name.span().pest(),
            )));
        }
    };

    let macro_def = info
        .macros
        .get(&(
            ArcIntern::clone(macro_access),
            ArcIntern::clone(&macro_call.name),
        ))
        .unwrap();

    Ok(match &**macro_def {
        crate::Macro::Splice { branches, after } => todo!(),
        crate::Macro::Builtin(call) => call(info, macro_call.arguments, id)?
            .into_iter()
            .map(|v| (v, Some(id)))
            .collect_vec(),
    })
}

#[cfg(test)]
mod tests {
    use crate::{macro_expansion::expand, parsing::parse};

    #[test]
    fn bruh() {
        let code = "
            .registers {
                a, b ← 3x3 builtin (90, 90)
            }

            loop:
                add 1 a
                print a What da heck
                solved-goto a loop

                add 89 b
                solved-goto b over
                goto loop

            over:

                halt b Poggers
        ";

        let parsed = match parse(code, &|_| unreachable!(), false) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };

        let expanded = match expand(parsed) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };

        println!("{expanded:?}");
    }
}
