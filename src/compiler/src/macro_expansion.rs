use std::{cell::OnceCell, mem};

use internment::ArcIntern;
use itertools::Itertools;
use pest::error::{Error, ErrorVariant};
use qter_core::WithSpan;

use crate::{BlockID, Code, Expanded, ExpansionInfo, Instruction, ParsedSyntax};

use super::parsing::Rule;

pub fn expand(parsed: ParsedSyntax) -> Result<Expanded, Box<Error<Rule>>> {
    todo!()
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
                changed.set(());
            }

            instruction
        })
        .flat_map(|v| {
            let block_info = info.block_info.get_mut(&id).unwrap();

            let span = v.span().to_owned();
            let instruction = v.into_inner();

            match instruction.0 {
                Instruction::Label(mut label) => {
                    if label.block.is_none() {
                        label.block = Some(id);
                        changed.set(());
                    }

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
                                    message: format!(
                                        "Cannot shadow a `.define` in the same scope!"
                                    ),
                                },
                                define.name.span().pest(),
                            )))];
                        }
                    }

                    block_info.defines.push(define);
                    changed.set(());

                    vec![]
                }
                Instruction::Registers(decl) => match block_info.registers {
                    Some(_) => {
                        return vec![Err(Box::new(Error::new_from_span(
                            ErrorVariant::CustomError {
                                message: format!(
                                    "Cannot have multiple register declarations in the same scope!"
                                ),
                            },
                            span.pest(),
                        )))];
                    }

                    None => {
                        block_info.registers = Some(decl);
                        changed.set(());
                        vec![]
                    }
                },
                Instruction::Code(code) => match expand_code(id, info, code) {
                    Ok(v) => v
                        .into_iter()
                        .map(|v| Ok(WithSpan::new(v, span.to_owned())))
                        .collect_vec(),
                    Err(e) => vec![Err(e)],
                },
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
) -> Result<Vec<(Instruction, Option<BlockID>)>, Box<Error<Rule>>> {
    let macro_call = match code {
        Code::Primitive(v) => return Ok(vec![(Instruction::Code(Code::Primitive(v)), Some(id))]),
        Code::Macro(v) => v,
    };

    let macro_access = match info.available_macros.get(&(
        macro_call.name.span().source(),
        ArcIntern::clone(&*macro_call.name),
    )) {
        Some(v) => v,
        None => {
            return Err(Box::new(Error::new_from_span(
                ErrorVariant::CustomError {
                    message: format!("Macro was not found in this scope"),
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
