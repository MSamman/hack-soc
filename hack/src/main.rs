use io::Write;
use std::fs::{self, OpenOptions};
use std::{io, path::PathBuf};

use anyhow::Result;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum, error::ErrorKind};
use hack::assembler::assemble;
use hack::board::Board;
use hack::programmer;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
enum OutKind {
    Stdout,
    File,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
enum Format {
    Hack,
    Bin,
}

#[derive(Args)]
struct FileInputs {
    asm_path: PathBuf,

    #[arg(short('b'), long("board"))]
    board_config_path: Option<PathBuf>,
}

impl FileInputs {
    fn read_asm(&self) -> Result<String> {
        let asm_string = fs::read_to_string(&self.asm_path)?;
        Ok(asm_string)
    }

    fn select_board(&self) -> Result<Board<'_>> {
        match &self.board_config_path {
            None => Ok(Board::default()),
            Some(p) => {
                let config_str = fs::read_to_string(p)?;
                let config_str = config_str.leak();
                Board::parse_config(config_str)
            }
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    Assemble {
        #[command(flatten)]
        input: FileInputs,

        #[arg(short, long, value_enum, default_value_t = OutKind::Stdout)]
        out: OutKind,

        #[arg(long, value_enum, default_value_t = Format::Hack)]
        format: Format,

        #[arg(long, value_name = "PATH")]
        file: Option<PathBuf>,
    },
    Load {
        #[command(flatten)]
        input: FileInputs,

        #[arg(long, value_name = "STRING")]
        port: String,
    },
    Ports {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // let board = match board_config_path {
    //     None => assembler::board::Board::default(),
    //     Some(path) => {
    //         let board_config = std::fs::read_to_string(board_config_path)?;
    //         assembler::board::load(board_config.as_str())?
    //     }
    // };
    // let asm_code = std::fs::read_to_string(target_file)?;
    // let hack_string = assemble(target_file, board)?;

    match &cli.command {
        Commands::Assemble {
            input,
            out,
            format,
            file,
            ..
        } => {
            let board = input.select_board()?;
            let asm_code = input.read_asm()?;
            println!("Assembling {} for {board:?}", input.asm_path.display());

            let bitcodes = assemble(&asm_code, &board)?;

            match out {
                OutKind::File => {
                    let default_name =  input.asm_path.with_extension(
                        match format {
                        Format::Hack => "hack",
                        Format::Bin => "bin",
                        }
                    );
                    let out_path = file.as_deref().unwrap_or(default_name.as_path());
                    println!("Writing to {out_path:?}...");

                    let mut out_file =
                        OpenOptions::new().write(true).create(true).truncate(true).open(out_path)?;

                    match format {
                        Format::Hack => {
                            write!(out_file, "{bitcodes}")?;
                        }
                        Format::Bin => {
                            bitcodes.write_bin(&mut out_file)?;
                        }
                    }
                }
                OutKind::Stdout => {
                    if file.is_some() {
                        Cli::command()
                            .error(
                                ErrorKind::ArgumentConflict,
                                "--file cannot be used with --out stdout",
                            )
                            .exit();
                    }
                    print!("{bitcodes}");
                }
            }
        }
        Commands::Ports {} => {
            programmer::print_ports();
        }
        Commands::Load { input, port } => {
            let board = input.select_board()?;
            let asm_code = input.read_asm()?;
            println!("Loading {} for {board:?}", input.asm_path.display());

            let bitcodes = assemble(&asm_code, &board)?;
            programmer::process_command(
                port,
                programmer::SerialCommand::Load {
                    words: bitcodes.words(),
                },
            )?;
        }
    }

    Ok(())
}
