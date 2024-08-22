use std::{
    env::args,
    process::exit,
    sync::mpsc,
    thread,
};

use gbemu::{
    display::SCREEN_SIZE,
    emulator::Emulator,
    msg::{ButtonState, EmulatorMsg, UserMsg},
};
use macroquad::prelude::*;
use miniquad::window::set_window_size;

const BLOCK_SZ: u32 = 5;
const WX: u32 = SCREEN_SIZE.0 as u32 * BLOCK_SZ;
const WY: u32 = SCREEN_SIZE.1 as u32 * BLOCK_SZ;

#[macroquad::main("[C]GB-Emulator")]
async fn main() {
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

    // Open ROM file and load it.
    let mut emu = match std::fs::read(&path) {
        Ok(rom) => match Emulator::new(&rom) {
            Ok(emu) => emu,
            Err(e) => {
                eprintln!("Emulator error: {:?}", e);
                exit(1);
            }
        },
        Err(e) => {
            eprintln!("cannot open file '{}': {:?}", path, e);
            exit(1);
        }
    };

    // Start the emulator and give it channels to send and recieve messages.
    let (user_tx, user_rx) = mpsc::channel::<UserMsg>();
    let (emu_tx, emu_rx) = mpsc::channel::<EmulatorMsg>();
    let handle = thread::spawn(move || {
        emu.run(user_rx, emu_tx);
    });

    let mut btn_state = ButtonState::default();

    // Configure window.
    prevent_quit();
    set_window_size(WX, WY);

    loop {
        // Handle events
        //-----------------------------------------------------------
        if is_key_pressed(KeyCode::Escape) || is_quit_requested() {
            break;
        }

        let new_state = get_button_state();
        if new_state != btn_state {
            btn_state = new_state;
            user_tx.send(UserMsg::Buttons(btn_state)).unwrap();
        }

        // Get frame
        user_tx.send(UserMsg::GetFrame).unwrap();
        let frame = match emu_rx.recv() {
            Ok(EmulatorMsg::NewFrame(f)) => f,
            _ => break,
        };

        // Get clock speed
        // user_tx.send(UserMsg::GetFrequency).unwrap();
        // let freq = match emu_rx.recv() {
        //     Ok(EmulatorMsg::Frequency(f)) => f,
        //     _ => break,
        // };

        // Draw stuff
        //-----------------------------------------------------------
        clear_background(BLACK);

        for y in 0..SCREEN_SIZE.1 {
            for x in 0..SCREEN_SIZE.0 {
                let (r, g, b) = frame.get(x, y).to_f32_triple();
                let col = Color { r, g, b, a: 1.0 };

                let px = x as f32 * BLOCK_SZ as f32;
                let py = y as f32 * BLOCK_SZ as f32;

                draw_rectangle(px, py, BLOCK_SZ as f32, BLOCK_SZ as f32, col);
            }
        }

        next_frame().await
    }

    user_tx.send(UserMsg::Shutdown).unwrap();
    matches!(emu_rx.recv(), Ok(EmulatorMsg::ShuttingDown));

    handle.join().unwrap();
}

fn get_button_state() -> ButtonState {
    ButtonState {
        a: is_key_down(KeyCode::Z),
        b: is_key_down(KeyCode::X),
        select: is_key_down(KeyCode::Enter),
        start: is_key_down(KeyCode::Backspace),
        up: is_key_down(KeyCode::W) || is_key_down(KeyCode::Up),
        down: is_key_down(KeyCode::S) || is_key_down(KeyCode::Down),
        left: is_key_down(KeyCode::A) || is_key_down(KeyCode::Left),
        right: is_key_down(KeyCode::D) || is_key_down(KeyCode::Right),
    }
}
