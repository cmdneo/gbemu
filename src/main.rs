mod gui;

use std::{env::args, process::exit};

fn main() {
    let path = match args().count() {
        2 => args().nth(1).unwrap(),

        _ => {
            eprintln!(
                "Usage: {} <rom-file>",
                args().next().unwrap_or("gbemu".to_string())
            );

            exit(1);
        }
    };

    // Open ROM file and load it into the emulator.
    let emu = match std::fs::read(&path) {
        Ok(rom) => match gbemu::Emulator::new(rom) {
            Ok(emu) => emu,
            Err(e) => {
                eprintln!("Emulator error: {e:?}");
                exit(1);
            }
        },
        Err(e) => {
            eprintln!("cannot open file '{path}': {e:?}");
            exit(1);
        }
    };

    let mut gui = gui::EmulatorGui::new(emu);

    eprint_help_text();
    gui.main_loop();
    eprintln!("\nQuit.");
}

fn eprint_help_text() {
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
