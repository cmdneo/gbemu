use std::{sync::mpsc, thread};

use sdl3::{
    audio,
    event::Event,
    hint,
    keyboard::{KeyboardState, Scancode},
    pixels::Color,
    render::{Canvas, FRect},
    video::Window,
    EventPump,
};

use gbemu::{Emulator, Reply, Request, FREQUENCY, SCREEN_RESOLUTION};

const BLOCK_SZ: u32 = 4;
const WX: u32 = SCREEN_RESOLUTION.0 as u32 * BLOCK_SZ;
const WY: u32 = SCREEN_RESOLUTION.1 as u32 * BLOCK_SZ;

const AUDIO_CONFIG: audio::AudioSpec = audio::AudioSpec {
    freq: Some(44100),
    channels: Some(2),
    format: Some(audio::AudioFormat::f32_sys()),
};

pub struct EmulatorGui {
    request_tx: mpsc::Sender<Request>,
    reply_rx: mpsc::Receiver<Reply>,

    audio: Option<EmulatorAudio>,
    handle: Option<thread::JoinHandle<()>>,
    running: bool,
}

struct EmulatorAudio {
    audio_ctrl_tx: mpsc::Sender<u32>,
    audio_data_rx: mpsc::Receiver<Box<[f32]>>,
}

impl audio::AudioCallback<f32> for EmulatorAudio {
    fn callback(&mut self, stream: &mut audio::AudioStream, _requested: i32) {
        // We need to adjust sampling period dynamically because the software
        // cannot exactly match the hardware timing and fractional periods are
        // not supported by the emulator. calc_sampling_period does that.
        let period = calc_sampling_period(stream);
        self.audio_ctrl_tx.send(period).unwrap();
        stream
            .put_data_f32(&self.audio_data_rx.recv().unwrap())
            .unwrap();
    }
}

impl EmulatorGui {
    pub fn new(mut emulator: Emulator) -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (reply_tx, reply_rx) = mpsc::channel();
        let (audio_ctrl_tx, audio_ctrl_rx) = mpsc::channel();
        let (audio_data_tx, audio_data_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            emulator.run(request_rx, reply_tx, audio_ctrl_rx, audio_data_tx);
        });

        Self {
            request_tx,
            reply_rx,
            audio: Some(EmulatorAudio {
                audio_ctrl_tx,
                audio_data_rx,
            }),
            handle: Some(handle),
            running: false,
        }
    }

    /// Run the emulator and return saved state of the emulator(if requested).
    pub fn main_loop(&mut self, save_state: bool) -> Option<Box<[u8]>> {
        hint::set(hint::names::RENDER_VSYNC, "1");
        let sdl_ctx = sdl3::init().unwrap();
        let video_sys = sdl_ctx.video().unwrap();
        let audio_sys = sdl_ctx.audio().unwrap();

        self.send(Request::Start);
        self.send(Request::GetTitle);
        self.running = true;
        let Reply::Title(rom_title) = self.recieve() else {
            panic!("invalid title reply")
        };

        let window = video_sys
            .window(&format!("gbemu - {rom_title}"), WX, WY)
            .position_centered()
            .build()
            .unwrap();

        let stream = audio_sys
            .open_playback_stream(&AUDIO_CONFIG, self.audio.take().unwrap())
            .unwrap();
        stream.resume().unwrap();

        let mut canvas = window.into_canvas();
        let mut event_pump = sdl_ctx.event_pump().unwrap();

        while self.running {
            self.update(&mut event_pump);
            self.draw(&mut canvas);
        }

        // Erase frequency printed line.
        eprintln!("\r                             ");
        stream.pause().unwrap();
        self.send(Request::Shutdown { save_state });
        self.handle.take().unwrap().join().unwrap();

        match self.recieve() {
            Reply::ShuttingDown(s) => s,
            _ => panic!("invalid shutdown reply"),
        }
    }

    fn update(&mut self, event_pump: &mut EventPump) {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => self.running = false,
                Event::KeyDown {
                    scancode: Some(Scancode::Space),
                    ..
                } => self.send(Request::CyclePalette),
                _ => (),
            }
        }

        self.update_keystate(event_pump);

        self.send(Request::GetFrequency);
        let Reply::Frequency(freq) = self.recieve() else {
            panic!("invalid reply")
        };
        eprint!("\r=> {:.3} MHz", freq / 1e6);
    }

    fn update_keystate(&self, event_pump: &EventPump) {
        let s = KeyboardState::new(event_pump);
        let pressed = |scancode| s.is_scancode_pressed(scancode);
        let btn_state = gbemu::ButtonState {
            a: pressed(Scancode::Z),
            b: pressed(Scancode::X),
            select: pressed(Scancode::Return),
            start: pressed(Scancode::Backspace),
            up: pressed(Scancode::W) || pressed(Scancode::Up),
            down: pressed(Scancode::S) || pressed(Scancode::Down),
            left: pressed(Scancode::A) || pressed(Scancode::Left),
            right: pressed(Scancode::D) || pressed(Scancode::Right),
        };

        self.send(Request::UpdateButtonState(btn_state));
    }

    fn draw(&self, canvas: &mut Canvas<Window>) {
        self.send(Request::GetVideoFrame);
        let Reply::VideoFrame(pixels) = self.recieve() else {
            panic!("invalid reply")
        };

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        for y in 0..SCREEN_RESOLUTION.1 {
            for x in 0..SCREEN_RESOLUTION.0 {
                let gbemu::Color { r, g, b } = pixels.get(x, y);
                let rect = FRect {
                    x: x as f32 * BLOCK_SZ as f32,
                    y: y as f32 * BLOCK_SZ as f32,
                    w: BLOCK_SZ as f32,
                    h: BLOCK_SZ as f32,
                };

                canvas.set_draw_color(Color::RGB(r, g, b));
                canvas.fill_rect(rect).unwrap();
            }
        }

        canvas.present();
    }

    fn send(&self, request: Request) {
        self.request_tx.send(request).unwrap()
    }

    fn recieve(&self) -> Reply {
        self.reply_rx.recv().unwrap()
    }
}

fn calc_sampling_period(stream: &audio::AudioStream) -> u32 {
    let audio::AudioSpec {
        freq: Some(freq),
        channels: Some(channels),
        ..
    } = stream.get_format().unwrap().1.unwrap()
    else {
        panic!("cannot retrieve audio format")
    };

    const MAX_PLAYBACK_IN_SECS: f64 = 0.01;
    let nsamples = stream.queued_bytes().unwrap() / channels / size_of::<f32>() as i32;
    let playback = nsamples as f64 / freq as f64;
    let exceeds = playback / MAX_PLAYBACK_IN_SECS;
    let period = FREQUENCY as f64 / freq as f64;

    // Warn and stop sampling if queueing up too many
    // samples which will cause high memory usage and audio latency.
    if playback > 10.0 * MAX_PLAYBACK_IN_SECS {
        eprintln!("warning: audio lag too many samples queued");
        return 0;
    }

    // Period is increased from the ideal by how many times playback
    // exceeds MAX_PLAYBACK, this is simple and handles overruns.
    // We floor the period so that we sample at a slightly faster rate to
    // avoid underruns which causes audible pops and choppy audio.
    // For the current AUDIO_CONFIG this method works fine, change if needed.
    (period + exceeds).floor() as u32
}
