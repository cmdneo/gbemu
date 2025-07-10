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
    gui.main_loop();
}
