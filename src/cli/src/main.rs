use std::{
    fmt::{Display, Formatter},
    fs, io,
    path::PathBuf,
};

use clap::{ArgAction, Parser, ValueEnum};
use color_eyre::{
    eyre::{OptionExt, eyre},
    owo_colors::OwoColorize,
};
use compiler::compile;
use internment::ArcIntern;
use interpreter::{ActionPerformed, ExecutionState, Interpreter, PausedState, PuzzleState};
use itertools::Itertools;
use qter_core::{
    I, Int,
    architectures::{Algorithm, Permutation},
    table_encoding::{decode_table, encode_table},
};

#[derive(Clone, Copy, ValueEnum)]
enum WrongSolvedGoto {
    /// Do not try to retrieve cube state from the robot and instead use a simulated cube state.
    Cheat,
    /// If the program returns an incorrect cube state, quit the program with an error.
    Quit,
    /// Print an error, but continue running with the new cube state
    PrintAccept,
    /// Print an error and ignore the cube state, using the simulated one instead
    PrintIgnore,
    /// Continue running with the new cube state without printing an error
    SilentAccept,
}

impl Display for WrongSolvedGoto {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        f.write_str(match self {
            WrongSolvedGoto::Cheat => "cheat",
            WrongSolvedGoto::Quit => "quit",
            WrongSolvedGoto::PrintAccept => "print-accept",
            WrongSolvedGoto::PrintIgnore => "print-ignore",
            WrongSolvedGoto::SilentAccept => "silent-accept",
        })
    }
}

/// Compiles and interprets qter programs
#[derive(Parser)]
#[command(version, about)]
enum Commands {
    /// Compile a QAT file to Q
    Compile {
        /// Which file to compile; must be a .q file
        file: PathBuf,
    },
    /// Interpret a QAT or a Q file
    Interpret {
        /// Which file to interpret; must be a .qat or .q file
        file: PathBuf,
        /// The level of execution trace to send to stderr. Can be set zero to three times.
        #[arg(short, action = ArgAction::Count)]
        trace_level: u8,
    },
    /// Step through a QAT or a Q program
    Debug {
        /// Which file to interpret; must be a .qat or .q file
        file: PathBuf,
    },
    /// Evaluate unit tests in a QAT program
    Test {
        /// Which file to test; must be a .qat file
        file: PathBuf,
    },
    /// Interpret a Qter program using a cube-solving robot
    ///
    /// The program inputted with `command` should receive signals through stdin and output signals through stdout.
    ///
    /// The Qter program may only use a single puzzle, and it may only be a 3x3 unless `--solved-goto` is set to `cheat`. Feel free to PR if you have a different robot.
    Robot {
        /// The program to control the robot along with arguments
        ///
        /// When the robot is ready to perform a new algorithm, the program must print the value of `--ready` followed by a line break through stdout. Then if it receives the `--perform` signal, the program will receive space separated sequence of cube moves to perform followed by a line break. If it receives the value of `--get-cube-state` followed by a line break, it will run the computer vision and print the cube state to stdout in the format of rob-twophase (https://github.com/efrantar/rob-twophase/blob/d245031257d52b2663c5790c5410ef30aefd775f/src/face.h#L29).
        ///
        /// If the robot does not have computer vision, you can use the option `--solved-goto cheat` to make Qter use a simulated cube state.
        command: String,
        /// How to handle the cube state returned by the robot not matching the simulated cube state
        #[arg(long, short, default_value_t = WrongSolvedGoto::PrintAccept)]
        solved_goto: WrongSolvedGoto,
        /// The signal to send to the robot command's stdin to perform an algorithm
        #[arg(long, short, default_value = "p")]
        perform: String,
        /// The signal to send to the robot command's stdin to print the cube state through stdout
        #[arg(long, short, default_value = "s")]
        get_cube_state: String,
        /// The signal sent by the robot command's stdout signalling that it is ready to perform an algorithm
        #[arg(long, short, default_value = "r")]
        ready: String,
    },
    #[cfg(debug_assertions)]
    /// Compress an algorithm table into the special format (This subcommand will not be visible in release mode)
    Compress {
        /// The input alg table
        input: PathBuf,
        /// The output compressed data
        output: PathBuf,
    },
    #[cfg(debug_assertions)]
    /// Print the contents of a compressed algorithm table to stdout (This subcommand will not be visible in release mode)
    Dump {
        /// The input alg table
        input: PathBuf,
    },
}

fn main() -> color_eyre::Result<()> {
    let args = Commands::parse();

    match args {
        Commands::Compile { file: _ } => todo!(),
        Commands::Interpret { file, trace_level } => {
            let program = match file.extension().and_then(|v| v.to_str()) {
                Some("q") => todo!(),
                Some("qat") => {
                    let qat = fs::read_to_string(file)?;

                    compile(&qat, |name| {
                        let path = PathBuf::from(name);

                        if path.ancestors().count() > 1 {
                            // Easier not to implement relative paths and stuff
                            return Err("Imported files must be in the same path".to_owned());
                        }

                        match fs::read_to_string(path) {
                            Ok(s) => Ok(ArcIntern::from(s)),
                            Err(e) => Err(e.to_string()),
                        }
                    })?
                }
                _ => {
                    return Err(eyre!(
                        "The file {file:?} must have an extension of `.qat` or `.q`."
                    ));
                }
            };

            let interpreter: Interpreter<Permutation> = Interpreter::new(program);

            if trace_level == 0 {
                interpret(interpreter)?;
            } else {
                interpret_traced(interpreter, trace_level)?;
            }
        }
        Commands::Debug { file: _ } => todo!(),
        Commands::Test { file: _ } => todo!(),
        Commands::Robot {
            command,
            solved_goto,
            perform,
            get_cube_state,
            ready,
        } => {}
        #[cfg(debug_assertions)]
        Commands::Compress { input, output } => {
            let data = fs::read_to_string(input)?;

            let to_encode = data
                .split('\n')
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(|alg| {
                    alg.split_whitespace()
                        .filter(|v| !v.is_empty())
                        .map(ArcIntern::from)
                        .collect_vec()
                })
                .collect_vec();

            // for alg in &to_encode {
            //     println!("{}", alg.iter().join(" "));
            // }

            let (data, _) =
                encode_table(&to_encode).ok_or_eyre("Too many unique generators, contact Henry")?;

            fs::write(output, data)?;
        }
        #[cfg(debug_assertions)]
        Commands::Dump { input } => {
            let data = fs::read(input)?;

            let decoded = decode_table(&data).ok_or_eyre("Could not decode the table")?;

            for moves in decoded {
                println!("{}", moves.iter().join(" "));
            }
        }
    }

    Ok(())
}

fn interpret<P: PuzzleState>(mut interpreter: Interpreter<P>) -> color_eyre::Result<()> {
    loop {
        let paused_state = interpreter.step_until_halt();

        let is_input_state = matches!(
            paused_state,
            PausedState::Input {
                max_input: _,
                puzzle_idx: _,
                register: _
            }
        );

        while let Some(message) = interpreter.messages().pop_front() {
            println!("{message}");
        }

        if is_input_state {
            give_number_input(&mut interpreter)?;
        } else {
            break Ok(());
        }
    }
}

fn give_number_input<P: PuzzleState>(
    interpreter: &mut Interpreter<P>,
) -> color_eyre::Result<(usize, Option<Algorithm>)> {
    loop {
        let mut number = String::new();
        io::stdin().read_line(&mut number)?;
        match number.parse::<Int<I>>() {
            Ok(value) => match interpreter.give_input(value) {
                Ok((puzzle_idx, exponentiated_alg)) => {
                    break Ok((puzzle_idx, exponentiated_alg));
                }
                Err(e) => println!("{e}"),
            },
            Err(_) => println!("Please input an integer"),
        }
    }
}

fn interpret_traced<P: PuzzleState>(
    mut interpreter: Interpreter<P>,
    trace_level: u8,
) -> color_eyre::Result<()> {
    let pad_amount = ((interpreter.program().instructions.len() - 1).ilog10() + 1) as usize;

    loop {
        if trace_level >= 3 {
            let mut program_counter = interpreter.program_counter().to_string();

            while program_counter.len() < pad_amount {
                program_counter.push(' ');
            }

            eprint!("{} | ", program_counter);
        }

        let action = interpreter.step();

        let mut should_give_input = false;
        let mut halted = false;

        match action {
            ActionPerformed::None => {
                if trace_level >= 2 {
                    eprintln!("Printing");
                }
            }
            ActionPerformed::Paused => {
                let is_input = matches!(
                    interpreter.execution_state(),
                    ExecutionState::Paused(PausedState::Input {
                        max_input: _,
                        puzzle_idx: _,
                        register: _
                    })
                );

                if is_input {
                    if trace_level >= 2 {
                        eprintln!("Accepting input");
                    }

                    should_give_input = true;
                } else {
                    if trace_level >= 2 {
                        eprintln!("Halting");
                    }

                    halted = true;
                }
            }
            ActionPerformed::Goto { instruction_idx: _ } => {
                if trace_level >= 3 {
                    eprintln!("Jumping");
                }
            }
            ActionPerformed::FailedSolvedGoto {
                puzzle_idx,
                facelets: _,
            } => {
                if trace_level >= 2 {
                    eprintln!("Inspect puzzle {puzzle_idx} - {}", "NOT TAKEN".red());
                }
            }
            ActionPerformed::SucceededSolvedGoto {
                puzzle_idx,
                facelets: _,
                instruction_idx: _,
            } => {
                if trace_level >= 2 {
                    eprintln!("Inspect puzzle {puzzle_idx} - {}", "TAKEN".green());
                }
            }
            ActionPerformed::AddedToTheoretical {
                puzzle_idx: register_idx,
                amt,
            } => {
                eprintln!("Theoretical {register_idx} += {amt}");
            }
            ActionPerformed::ExecutedAlgorithm {
                puzzle_idx,
                algorithm,
            } => {
                eprint!("Puzzle {puzzle_idx}:");

                for move_ in algorithm.move_seq() {
                    eprint!(" {move_}");
                }

                eprintln!();
            }
            ActionPerformed::Panicked => {
                eprintln!("{}", "Panicked!".red());
                halted = true;
            }
        }

        while let Some(interpreter_message) = interpreter.messages().pop_front() {
            println!("{interpreter_message}");
        }

        if halted {
            break Ok(());
        }

        if should_give_input {
            let (puzzle_idx, exponentiated_alg) = give_number_input(&mut interpreter)?;

            let Some(exponentiated_puzzle_alg) = exponentiated_alg else {
                continue;
            };

            eprint!("Puzzle {puzzle_idx}:");

            for move_ in exponentiated_puzzle_alg.move_seq() {
                eprint!(" {move_}");
            }

            eprintln!();
        }
    }
}
