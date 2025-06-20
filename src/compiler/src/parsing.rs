use crate::{
    BlockInfo, BlockInfoTracker, ExpansionInfo, builtin_macros::builtin_macros, lua::LuaMacros,
};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use chumsky::{
    extra::{Full, SimpleState},
    input::MapExtra,
    inspector::Inspector,
    prelude::*,
};
use internment::ArcIntern;
use itertools::Itertools;
use qter_core::{
    Extra, File, Int, MaybeErr, Span, U, WithSpan,
    architectures::{Architecture, puzzle_definition},
};

use crate::{BlockID, Macro, ParsedSyntax, Puzzle, RegistersDecl};

use super::Instruction;

static PRELUDE: LazyLock<ParsedSyntax> = LazyLock::new(|| {
    let prelude = File::from(include_str!("../../qter_core/prelude.qat"));

    let mut parsed_prelude = match parse(
        prelude.clone(),
        &|_| {
            panic!(
                "Prelude should not import files (because it's easier not to implement; message henry if you need this feature)"
            )
        },
        true,
    ) {
        Ok(v) => v,
        Err(e) => panic!("{e:?}"),
    };

    let builtin_macros = builtin_macros(&prelude.inner());
    parsed_prelude
        .expansion_info
        .available_macros
        .extend(builtin_macros.keys().map(|source_and_macro_name| {
            (
                source_and_macro_name.to_owned(),
                source_and_macro_name.0.clone(),
            )
        }));
    parsed_prelude.expansion_info.macros.extend(builtin_macros);

    parsed_prelude
});

type ExtraAndSyntax = Full<Rich<'static, char, Span>, SimpleState<ParsedSyntax>, ()>;

pub fn parse(
    qat: File,
    find_import: &impl Fn(&str) -> Result<ArcIntern<str>, String>,
    is_prelude: bool,
) -> Result<ParsedSyntax, Vec<Rich<'static, char, Span>>> {
    thread_local! {
        static PARSER: Boxed<'static, 'static, File, (), ExtraAndSyntax> = parser().boxed();
    }

    let zero_span = Span::new(qat.inner(), 0, 0);

    let lua_macros = match LuaMacros::new() {
        Ok(v) => v,
        Err(e) => {
            return Err(vec![Rich::custom(
                zero_span,
                format!("Failed to initialize the Lua runtime. {e}"),
            )]);
        }
    };

    let expansion_info = ExpansionInfo {
        block_counter: 1,
        block_info: BlockInfoTracker(HashMap::new()),
        macros: HashMap::new(),
        available_macros: HashMap::new(),
        lua_macros: HashMap::new(),
    };
    let code = Vec::new();

    let mut parsed_syntax = SimpleState(ParsedSyntax {
        expansion_info,
        code,
    });

    PARSER
        .with(|parser| parser.parse_with_state(qat.clone(), &mut parsed_syntax))
        .into_result()?;

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
        .insert(qat.inner(), lua_macros);

    Ok(parsed_syntax.0)
}

type ExtraAndState<S> = Full<Rich<'static, char, Span>, S, ()>;

fn parser() -> impl Parser<'static, File, (), ExtraAndSyntax> {
    group((
        shebang::<SimpleState<ParsedSyntax>>(),
        whitespace(),
        registers().with_state(()).map_with(
            |regs, data: &mut MapExtra<'_, '_, File, ExtraAndSyntax>| {
                if let MaybeErr::Some(regs) = regs {
                    let span = data.span();
                    data.state()
                        .code
                        .push(span.with((Instruction::Registers(regs), Some(BlockID(0)))));
                }
            },
        ),
    ))
    .to(())
}

fn shebang<S: Inspector<'static, File> + 'static>()
-> impl Parser<'static, File, (), ExtraAndState<S>> {
    any().repeated().delimited_by(just("#!"), just('\n')).to(())
}

fn req_whitespace<S: Inspector<'static, File> + 'static>()
-> impl Parser<'static, File, (), ExtraAndState<S>> {
    choice((
        just(' ').to(()),
        just('\t').to(()),
        any()
            .repeated()
            .delimited_by(just("--[["), just("]]--"))
            .to(()),
        any().repeated().delimited_by(just("--"), just('\n')).to(()),
    ))
    .repeated()
    .at_least(1)
}

fn whitespace<S: Inspector<'static, File> + 'static>()
-> impl Parser<'static, File, (), ExtraAndState<S>> {
    req_whitespace().or_not().to(())
}

fn nl<S: Inspector<'static, File> + 'static>() -> impl Parser<'static, File, (), ExtraAndState<S>> {
    just('\n')
        .separated_by(whitespace())
        .at_least(1)
        .allow_leading()
        .allow_trailing()
}

fn nlm<S: Inspector<'static, File> + 'static>() -> impl Parser<'static, File, (), ExtraAndState<S>>
{
    just('\n')
        .separated_by(whitespace())
        .allow_leading()
        .allow_trailing()
}

fn number<S: Inspector<'static, File> + 'static>()
-> impl Parser<'static, File, (), ExtraAndState<S>> {
    any()
        .filter(|c: &char| c.is_ascii_digit())
        .repeated()
        .at_least(1)
        .to(())
}

fn intu<S: Inspector<'static, File> + 'static>()
-> impl Parser<'static, File, MaybeErr<Int<U>>, ExtraAndState<S>> {
    number().validate(|(), data, emitter| match data.span().slice().parse() {
        Ok(v) => MaybeErr::Some(v),
        Err(e) => {
            emitter.emit(Rich::custom(
                data.span(),
                format!("Could not parse as an integer: {e}"),
            ));
            MaybeErr::None
        }
    })
}

fn ident<S: Inspector<'static, File> + 'static>()
-> impl Parser<'static, File, WithSpan<ArcIntern<str>>, ExtraAndState<S>> {
    let special_char = choice((
        just('{').to(()),
        just('}').to(()),
        just('.').to(()),
        just(':').to(()),
        just('$').to(()),
        just("--").to(()),
        just(',').to(()),
        just("<-").to(()),
        just('←').to(()),
        just('\n').to(()),
        just('(').to(()),
        just(')').to(()),
        just('!').to(()),
        just('"').to(()),
        req_whitespace(),
    ));

    choice((
        group((special_char.not(), any()))
            .repeated()
            .at_least(1)
            .to_span()
            .filter(|span: &Span| {
                span.slice().chars().any(|c| !c.is_ascii_digit()) && span.slice() != "lua"
            }),
        group((just('"').not(), any()))
            .repeated()
            .to_span()
            .delimited_by(just('"'), just('"')),
    ))
    .map(|v: Span| WithSpan::new(ArcIntern::from(v.slice()), v))
}

fn registers() -> impl Parser<'static, File, MaybeErr<RegistersDecl>, Extra> {
    group((
        just(".registers"),
        whitespace(),
        just("{"),
        register_decl()
            .separated_by(nl())
            .at_least(1)
            .allow_leading()
            .allow_trailing()
            .collect::<MaybeErr<Vec<_>>>(),
        just("}"),
    ))
    .delimited_by(nlm(), nlm())
    .map(|(_, (), _, puzzles, _)| {
        puzzles.map(|puzzles| RegistersDecl {
            puzzles,
            block_id: BlockID(0),
        })
    })
}

fn register_decl() -> impl Parser<'static, File, MaybeErr<Puzzle>, Extra> {
    choice((register_decl_switchable(), register_decl_unswitchable()))
}

fn register_decl_unswitchable() -> impl Parser<'static, File, MaybeErr<Puzzle>, Extra> {
    group((
        ident()
            .separated_by(just(',').delimited_by(whitespace(), whitespace()))
            .at_least(1)
            .collect::<Vec<_>>(),
        choice((just("<-").to(()), just('←').to(()))).delimited_by(whitespace(), whitespace()),
        register_architecture(),
    ))
    .validate(|(mut names, (), archs), data, emitter| {
        archs
            .map(|archs| match archs {
                PuzzleUnnamed::Theoretical { order } => {
                    if names.len() == 1 {
                        MaybeErr::Some(Puzzle::Theoretical {
                            name: names.pop().unwrap(),
                            order,
                        })
                    } else {
                        emitter.emit(Rich::custom(
                            data.span(),
                            format!("Expected one name whereas {} were provided.", names.len()),
                        ));

                        MaybeErr::None
                    }
                }
                PuzzleUnnamed::Real { architecture } => {
                    if architecture.registers().len() == names.len() {
                        MaybeErr::Some(Puzzle::Real {
                            architectures: vec![(names, architecture)],
                        })
                    } else {
                        emitter.emit(Rich::custom(
                            data.span(),
                            format!(
                                "Expected {} names whereas {} were provided.",
                                architecture.registers().len(),
                                names.len()
                            ),
                        ));

                        MaybeErr::None
                    }
                }
            })
            .flatten()
    })
}

#[derive(Clone, Debug)]
enum PuzzleUnnamed {
    Theoretical {
        order: WithSpan<Int<U>>,
    },
    Real {
        architecture: WithSpan<Arc<Architecture>>,
    },
}

fn algorithm() -> impl Parser<'static, File, Vec<Span>, Extra> {
    ident()
        .to_span()
        .separated_by(whitespace())
        .at_least(1)
        .collect()
}

fn register_architecture() -> impl Parser<'static, File, MaybeErr<PuzzleUnnamed>, Extra> {
    choice((
        group((
            just("theoretical"),
            whitespace(),
            intu().map_with(|v, extra| v.map(|v| extra.span().with(v))),
        ))
        .map(|(_, (), order)| order.map(|order| PuzzleUnnamed::Theoretical { order })),
        group((
            puzzle_definition(),
            whitespace(),
            just("builtin"),
            whitespace(),
            choice((
                intu().map(|v| v.map(|v| vec![v])),
                intu()
                    .separated_by(just(",").delimited_by(nlm(), nlm()))
                    .at_least(1)
                    .collect::<MaybeErr<Vec<_>>>()
                    .delimited_by(just("("), just(")")),
            ))
            .map_with(|v, data| data.span().with(v)),
        ))
        .validate(
            |(def, (), _, (), orders), data, emitter| orders.spanspose().map(|orders| if let Some(arch) = def.get_preset(&orders) { MaybeErr::Some(PuzzleUnnamed::Real {
                architecture: data.span().with(arch),
            }) } else {
                emitter.emit(Rich::custom(
                                orders.span().clone(),
                                "There does not exist a preset architecture with the given orders.",
                            ));
                            MaybeErr::None
            },
        ).flatten()),
        group((
            puzzle_definition(),
            whitespace(),
            choice((
                algorithm().map(|v| vec![v]),
                algorithm()
                    .separated_by(just(",").delimited_by(nlm(), nlm()))
                    .allow_trailing()
                    .at_least(1)
                    .collect()
                    .delimited_by(just("("), just(")")),
            ))
            .map_with(|v, data| data.span().with(v)),
        ))
        .validate(|(def, (), algs), data, emitter| {
            match Architecture::new(Arc::clone(&def.perm_group), &algs) {
                Ok(arch) => MaybeErr::Some(PuzzleUnnamed::Real {
                    architecture: data.span().with(Arc::new(arch)),
                }),
                Err(bad_generator) => {
                    emitter.emit(Rich::custom(bad_generator.clone(), format!("This generator does not exist in the given permutation group. The options are: {}", def.perm_group.generators().map(|(name, _)| name).join(&ArcIntern::from(", ")))));

                    MaybeErr::None
                },
            }
        }),
    ))
}

fn register_decl_switchable() -> impl Parser<'static, File, MaybeErr<Puzzle>, Extra> {
    register_decl_unswitchable()
        .validate(|v, data, emitter| {
            v.map(|v| match v {
                Puzzle::Theoretical { name: _, order: _ } => {
                    emitter.emit(Rich::custom(
                        data.span(),
                        "Theoretical architectures cannot be switchable.",
                    ));
                    MaybeErr::None
                }
                Puzzle::Real { architectures } => MaybeErr::Some(architectures),
            })
            .flatten()
        })
        .separated_by(nl())
        .allow_leading()
        .allow_trailing()
        .at_least(1)
        .collect::<MaybeErr<Vec<_>>>()
        .delimited_by(just('('), just(')'))
        .map(|v| {
            v.map(|v| Puzzle::Real {
                architectures: v
                    .into_iter()
                    .reduce(|mut a, b| {
                        a.extend_from_slice(&b);
                        a
                    })
                    .unwrap(),
            })
        })
}

fn merge_files(
    importer: &mut ParsedSyntax,
    importer_contents: &ArcIntern<str>,
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
                ArcIntern::clone(importer_contents),
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

enum Statement<'a> {
    Macro {
        name: WithSpan<ArcIntern<str>>,
        def: WithSpan<Macro>,
    },
    Instruction(WithSpan<Instruction>),
    LuaBlock(&'a str),
    Import(WithSpan<&'a str>),
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;
    use internment::ArcIntern;
    use qter_core::File;

    use super::{ident, number, parse, registers};

    #[test]
    fn test_number() {
        number::<()>().parse(File::from("123")).unwrap();
        number::<()>().parse(File::from("12398263596868928956891896286935689869218695689689297479561963469856981968423679569173479159")).unwrap();

        assert!(number::<()>().parse(File::from("")).has_errors());
        assert!(number::<()>().parse(File::from("3x3")).has_errors());
        assert!(number::<()>().parse(File::from("0.12")).has_errors());
        assert!(number::<()>().parse(File::from("-11")).has_errors());
        assert!(number::<()>().parse(File::from("-11")).has_errors());
    }

    #[test]
    fn test_ident() {
        ident::<()>().parse(File::from("a")).unwrap();
        ident::<()>().parse(File::from("A")).unwrap();
        ident::<()>().parse(File::from("3x3")).unwrap();
        ident::<()>().parse(File::from("thingy")).unwrap();
        ident::<()>().parse(File::from("pluah")).unwrap();
        ident::<()>().parse(File::from("->")).unwrap();
        ident::<()>().parse(File::from("\"345\"")).unwrap();
        ident::<()>().parse(File::from("\"lua\"")).unwrap();

        assert!(ident::<()>().parse(File::from("345")).has_errors());
        assert!(ident::<()>().parse(File::from("lua")).has_errors());
        assert!(ident::<()>().parse(File::from("thing<-thing")).has_errors());
        assert!(ident::<()>().parse(File::from("aa.aa")).has_errors());
        assert!(ident::<()>().parse(File::from("!aaaa")).has_errors());
        assert!(ident::<()>().parse(File::from("aaaa)")).has_errors());
    }

    #[test]
    fn test_registers() {
        let code = "
            .registers {
                a, b <- 3x3 builtin (90, 90)
                (
                    c, d ← 3x3 builtin (210, 24)
                    d, e, f ← 3x3 builtin (30, 30, 30)
                )
                f ← theoretical 90
                g, h ← 3x3 (U, D)
            }
        ";

        let errs = registers().parse(File::from(code)).into_errors();

        for err in &errs {
            println!("{err}; {:?}", err.span().line_and_col());
        }

        assert!(errs.is_empty());
    }

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

        match parse(File::from(code), &|_| Ok(ArcIntern::from("add 1 a")), false) {
            Ok(_) => {}
            Err(e) => panic!("{e:?}"),
        }
    }
}
