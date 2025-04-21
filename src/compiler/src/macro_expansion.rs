use std::{cell::OnceCell, mem};

use internment::ArcIntern;
use itertools::Itertools;
use pest::error::Error;
use qter_core::{WithSpan, mk_error};

use crate::{
    BlockID, Code, ExpandedCode, ExpandedCodeComponent, ExpansionInfo, Instruction, Macro,
    ParsedSyntax, TaggedInstruction,
};

use super::parsing::Rule;

pub fn expand(mut parsed: ParsedSyntax) -> Result<ExpandedCode, Box<Error<Rule>>> {
    // TODO: Logic of `after`
    while expand_block(BlockID(0), &mut parsed.expansion_info, &mut parsed.code)? {}

    Ok(ExpandedCode {
        block_info: parsed.expansion_info.block_info,
        expanded_code_components: parsed
            .code
            .into_iter()
            .map(|tagged_instruction| {
                let span = tagged_instruction.span().to_owned();
                let (instruction, maybe_block_id) = tagged_instruction.into_inner();

                let expanded = match instruction {
                    Instruction::Label(label) => ExpandedCodeComponent::Label(label),
                    Instruction::Code(Code::Primitive(primitive)) => {
                        ExpandedCodeComponent::Instruction(
                            Box::new(primitive),
                            maybe_block_id.unwrap(),
                        )
                    }
                    illegal => unreachable!("{illegal:?}"),
                };

                WithSpan::new(expanded, span)
            })
            .collect_vec(),
    })
}

/// Returns whether any changes were made
fn expand_block(
    block_id: BlockID,
    expansion_info: &mut ExpansionInfo,
    code: &mut Vec<WithSpan<TaggedInstruction>>,
) -> Result<bool, Box<Error<Rule>>> {
    // Will be set if anything is ever changed
    let changed = OnceCell::<()>::new();

    *code = mem::take(code)
        .into_iter()
        .map(|mut tagged_instruction| {
            let maybe_block_id = &mut tagged_instruction.1;
            if maybe_block_id.is_none() {
                *maybe_block_id = Some(block_id);
                let _ = changed.set(());
            }

            tagged_instruction
        })
        .flat_map(|tagged_instruction| {
            let span = tagged_instruction.span().to_owned();

            let (instruction, maybe_block_id) = tagged_instruction.into_inner();
            let block_id = maybe_block_id.unwrap();

            let block_info = expansion_info.block_info.0.get_mut(&block_id).unwrap();

            match instruction {
                Instruction::Label(mut label) => {
                    if label.maybe_block_id.is_none() {
                        label.maybe_block_id = Some(block_id);
                        let _ = changed.set(());
                    }

                    block_info.labels.push(label.clone());

                    vec![Ok(WithSpan::new(
                        (Instruction::Label(label), maybe_block_id),
                        span,
                    ))]
                }
                Instruction::Define(define) => {
                    for found_define in &block_info.defines {
                        if *found_define.name == *define.name {
                            return vec![Err(mk_error(
                                "Cannot shadow a `.define` in the same scope!",
                                define.name.span(),
                            ))];
                        }
                    }

                    block_info.defines.push(define);
                    let _ = changed.set(());

                    vec![]
                }
                Instruction::Registers(decl) => {
                    if block_info.registers.is_some() {
                        vec![Err(mk_error(
                            "Cannot have multiple register declarations in the same scope!",
                            span,
                        ))]
                    } else {
                        block_info.registers = Some(decl);
                        let _ = changed.set(());
                        vec![]
                    }
                }
                Instruction::Code(code) => {
                    match expand_code(block_id, expansion_info, code, &changed) {
                        Ok(tagged_instructions) => tagged_instructions
                            .into_iter()
                            .map(|tagged_instruction| {
                                Ok(WithSpan::new(tagged_instruction, span.clone()))
                            })
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
    block_id: BlockID,
    expansion_info: &mut ExpansionInfo,
    code: Code,
    changed: &OnceCell<()>,
) -> Result<Vec<TaggedInstruction>, Box<Error<Rule>>> {
    let macro_call = match code {
        Code::Primitive(prim) => {
            return Ok(vec![(
                Instruction::Code(Code::Primitive(prim)),
                Some(block_id),
            )]);
        }
        Code::Macro(mac) => mac,
    };

    let _ = changed.set(());

    let Some(macro_access) = expansion_info.available_macros.get(&(
        macro_call.name.span().source(),
        ArcIntern::clone(&*macro_call.name),
    )) else {
        return Err(mk_error(
            "Macro was not found in this scope",
            macro_call.name.span(),
        ));
    };

    let macro_def = expansion_info
        .macros
        .get(&(
            ArcIntern::clone(macro_access),
            ArcIntern::clone(&macro_call.name),
        ))
        .unwrap();

    Ok(match &**macro_def {
        Macro::UserDefined {
            branches: _,
            after: _,
        } => todo!(),
        Macro::Builtin(macro_fn) => macro_fn(expansion_info, macro_call.arguments, block_id)?
            .into_iter()
            .map(|instruction| (instruction, Some(block_id)))
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
                add a 1
                print \"What da heck\" a
                solved-goto a loop

                add b 89
                solved-goto b over
                goto loop

            over:

                halt \"Poggers\" b
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
