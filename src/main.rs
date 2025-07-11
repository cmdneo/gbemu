mod gui;

use std::{
    fmt::{Debug, Display},
    fs::File,
    io::Write,
    path::PathBuf,
    process::exit,
};

use clap::{arg, Parser, Subcommand};
use gbemu::Emulator;

#[derive(Parser)]
#[command(name = "gbemu", about = "Gameboy Emulator")]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Load a ROM into the emulator and run.
    #[command(arg_required_else_help = true)]
    Run {
        /// Gameboy ROM file
        rom_file: PathBuf,
        /// Save the emulator state into a save file on exit
        #[arg(long, value_name = "SAVE_FILE")]
        save_to: Option<PathBuf>,
    },

    /// Resume the emulator from a save file, on exit the new state is
    /// saved into the same file unless changed using options below.
    #[command(verbatim_doc_comment, arg_required_else_help = true)]
    Resume {
        /// Saved file
        save_file: PathBuf,
        /// Do not save new state into the current save file
        #[arg(long, conflicts_with = "save_to")]
        no_save: bool,
        /// Save new state into the given file while leaving the
        /// current save file unchanged
        #[arg(long, value_name = "SAVE_FILE", conflicts_with = "no_save")]
        save_to: Option<PathBuf>,
    },

    /// Extract ROM from the save file and save it into the given file
    ExtractRom {
        /// Saved file
        save_file: PathBuf,
        /// New ROM file
        rom_file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let (emulator, save_to) = match cli.commands {
        Commands::Run { rom_file, save_to } => (
            Emulator::from_rom(read_or_exit(&rom_file, "ROM file")),
            save_to,
        ),

        Commands::Resume {
            save_file,
            no_save,
            save_to,
        } => {
            let save_to = if no_save {
                None
            } else if save_to.is_some() {
                save_to
            } else {
                Some(save_file.clone())
            };
            (
                Emulator::from_saved(read_or_exit(&save_file, "save file")),
                save_to,
            )
        }

        Commands::ExtractRom {
            save_file,
            rom_file,
        } => {
            match Emulator::rom_from_saved(read_or_exit(&save_file, "save file")) {
                Ok(rom) => {
                    write_or_exit(&rom_file, "ROM file", &rom);
                    eprintln!("ROM saved saved to file {rom_file:?}.");
                }
                Err(e) => {
                    err_exit("Decoding save file failed", e);
                }
            }
            return;
        }
    };

    if let Some(path) = &save_to {
        if !path.is_file() && path.exists() {
            err_exit(format!("{path:?} is not a file"), "InvalidArgument")
        }
    }
    if let Err(e) = emulator {
        err_exit("Failed to initialize emulator", e);
    }

    let mut gui = gui::EmulatorGui::new(emulator.unwrap());
    eprint_keybindings();

    if let Some(path) = save_to {
        let saved = gui.main_loop(true);
        write_or_exit(&path, "save file", &saved.unwrap());
        eprintln!("Game state saved to file {path:?}.");
    } else {
        assert!(gui.main_loop(false).is_none());
    }

    eprintln!("Quit.");
}

fn read_or_exit(path: &PathBuf, err_name: &str) -> Vec<u8> {
    match std::fs::read(path) {
        Ok(ret) => ret,
        Err(e) => err_exit(
            format!("Cannot open {err_name} {path:?} for reading"),
            e.kind(),
        ),
    }
}

fn write_or_exit(path: &PathBuf, err_name: &str, data: &[u8]) {
    match File::create(path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(data) {
                err_exit(format!("Write to {err_name} {path:?} failed"), e.kind());
            }
        }
        Err(e) => {
            err_exit(
                format!("Cannot open {err_name} {path:?} for writing"),
                e.kind(),
            );
        }
    }
}

fn err_exit<M: Display, E: Debug>(msg: M, err: E) -> ! {
    eprintln!("{msg}.");
    eprintln!("Error: {err:?}.");
    exit(1);
}

fn eprint_keybindings() {
    eprintln!("--------Emulator Keybindings--------");
    eprintln!("START  : backspace");
    eprintln!("SELECT : return");
    eprintln!("A      : Z");
    eprintln!("B      : X");
    eprintln!("UP     : W/↑");
    eprintln!("DOWN   : S/↓");
    eprintln!("LEFT   : A/←");
    eprintln!("RIGHT  : D/→");
    eprintln!();

    eprintln!("--------Control Keybindings---------");
    eprintln!("Change palette  : space");
    eprintln!("Exit emulator   : escape");
    eprintln!();
}
