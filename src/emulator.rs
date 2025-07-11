use std::{
    sync::mpsc::{Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use crate::{
    cartridge::Cartidge,
    cpu::Cpu,
    log,
    mmu::Mmu,
    msg::{Reply, Request, VideoFrame},
    EmulatorErr,
};

pub struct Emulator {
    cpu: Cpu,
    /// Total T-cycles ticked since last `timer_reset`.
    tcycles: u64,
    /// Time when the timer was reset.
    start_time: Instant,
    /// Actual clock frequency achieved by the emulator
    real_frequency: f64,
    init_required: bool,
    is_running: bool,
    save_state: bool,
}

impl Emulator {
    pub fn from_rom(rom: Vec<u8>) -> Result<Self, EmulatorErr> {
        let cartidge = Cartidge::new(rom)?;
        let mmu = Mmu::new(cartidge);
        let cpu = Cpu::new(mmu);

        Ok(Self {
            cpu,
            tcycles: 0,
            real_frequency: 0.0,
            start_time: Instant::now(),
            init_required: true,
            is_running: false,
            save_state: false,
        })
    }

    pub fn from_saved(saved: Vec<u8>) -> Result<Self, EmulatorErr> {
        Ok(Self {
            cpu: load_save_file(&saved)?,
            tcycles: 0,
            real_frequency: 0.0,
            start_time: Instant::now(),
            init_required: false,
            is_running: false,
            save_state: false,
        })
    }

    pub fn rom_from_saved(saved: Vec<u8>) -> Result<Box<[u8]>, EmulatorErr> {
        Ok(load_save_file(&saved)?.mmu.cart.rom.clone())
    }

    /// Run it in a new thread and use channels to communicate with it
    /// information: buttons presses, frame requests and other commands.
    /// Send a [Request::Start] to actually start the emulator and run until
    /// [Request::Shutdown] is recieved.
    ///
    /// Parameters:  
    /// - `request_rx`   : For [Request] messages for controlling the emulator.
    /// - `reply_tx`     : For [Reply] messages (if any) for recieved messages.
    /// - `audio_ctrl_rx`: For starting a new audio sampling with the specified
    ///   sampling period and returning the previously accumulated samples,
    ///   a period of 0 stops sampling.
    /// - `audio_data_tx`: For recieving the accumulated audio data.
    pub fn run(
        &mut self,
        request_rx: Receiver<Request>,
        reply_tx: Sender<Reply>,
        audio_ctrl_rx: Receiver<u32>,
        audio_data_tx: Sender<Box<[f32]>>,
    ) {
        if !matches!(request_rx.recv().unwrap(), Request::Start) {
            panic!("Emulator not started yet, send [Request::Start] first.");
        }

        if self.init_required {
            self.init();
        }
        self.reset_timers();
        self.is_running = true;

        while self.is_running {
            // Run multiple steps in one burst for efficiency. Try not to
            // runmore than 0.005 seconds worth of cycles at once, otherwise,
            // requests for audio/video frames might get blocked for too long.
            // Max dots an instruction can take is 24 dots, thus:
            // 0.005 * FREQUENCY(=2^22) / 24 = 873, so run less than 873 steps.
            for _ in 0..777 {
                self.step();
            }

            self.handle_audio_flow(&audio_ctrl_rx, &audio_data_tx);
            self.handle_msgs(&request_rx, &reply_tx);
            self.manage_sleep_timer();
        }

        if !self.save_state {
            reply_tx.send(Reply::ShuttingDown(None)).unwrap();
            return;
        }

        // Remove video frame, clear audio samples and disable sampling before saving.
        self.cpu.mmu.ppu.remove_frame();
        self.cpu.mmu.apu.start_new_sampling(0);
        let saved = bincode::encode_to_vec(&self.cpu, bincode::config::standard()).unwrap();
        reply_tx
            .send(Reply::ShuttingDown(Some(saved.into_boxed_slice())))
            .unwrap();
    }

    /// Run a for a step each component.
    // Runs each component step-by-step.
    // In the real hardware eveything is synchronized by a master clock.
    // Here, we try to achieve the same effect by running each component
    // step-by-step. It is as if the CPU produces cycles and other components
    // (PPU and Timer) consume it.
    fn step(&mut self) {
        let mcycles = self.cpu.step();
        assert!(mcycles > 0);
        self.tcycles += mcycles as u64 * 4;
    }

    /// Handle user messages and respond to them(if required).
    fn handle_msgs(&mut self, request_rx: &Receiver<Request>, reply_tx: &Sender<Reply>) {
        let msg = match request_rx.try_recv() {
            Ok(msg) => msg,
            Err(TryRecvError::Empty) => return,
            Err(e) => panic!("message channel: {e:?}"),
        };

        match msg {
            Request::Start => panic!("already running"),

            Request::UpdateButtonState(btns) => {
                let (dpad, btns) = btns.to_internal_repr();
                self.cpu.mmu.update_joypad(dpad, btns)
            }

            Request::CyclePalette => self.cpu.mmu.ppu.cycle_palette(1),

            Request::GetVideoFrame => {
                let mut f = Box::new(VideoFrame::default());
                self.cpu.mmu.ppu.copy_frame(f.as_mut());
                reply_tx.send(Reply::VideoFrame(f)).unwrap()
            }

            Request::GetTitle => reply_tx
                .send(Reply::Title(self.cpu.mmu.cart.title.clone()))
                .unwrap(),

            Request::GetFrequency => reply_tx
                .send(Reply::Frequency(self.real_frequency))
                .unwrap(),

            Request::Shutdown { save_state } => {
                self.save_state = save_state;
                self.is_running = false;
            }

            Request::DebuggerStart => todo!(),
            Request::DebuggerStep => todo!(),
            Request::DebuggerStop => todo!(),
        }
    }

    fn handle_audio_flow(
        &mut self,
        audio_ctrl_rx: &Receiver<u32>,
        audio_data_tx: &Sender<Box<[f32]>>,
    ) {
        let period = match audio_ctrl_rx.try_recv() {
            Ok(p) => p,
            Err(TryRecvError::Empty) => return,
            Err(e) => panic!("audio channel: {e:?}"),
        };

        audio_data_tx
            .send(
                self.cpu
                    .mmu
                    .apu
                    .start_new_sampling(period)
                    .into_boxed_slice(),
            )
            .unwrap();
    }

    /// Initialize the registers and state, make it ready for execution.
    fn init(&mut self) {
        // Initial values for starting up the program in DMG mode as per:
        // https://gbdev.io/pandocs/Power_Up_Sequence.html
        self.cpu.pc.0 = 0x0100;
        self.cpu.sp.0 = 0xFFFE;
        self.cpu.mmu.joypad.write(0xCF);
        self.cpu.mmu.wram_idx = 1;
        self.cpu.mmu.ppu.bgp = 0xFC;
        self.cpu.mmu.ppu.fetcher.lcdc.write(0x91);
        self.cpu.mmu.ppu.stat.write(0x85);
    }

    fn manage_sleep_timer(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let executed = self.tcycles as f64 / self.cpu.frequency as f64;
        let ahead = executed - elapsed;

        self.real_frequency = self.tcycles as f64 / elapsed;
        if ahead > 0.0 {
            thread::sleep(Duration::from_secs_f64(ahead));
        }
    }

    fn reset_timers(&mut self) {
        self.tcycles = 0;
        self.start_time = Instant::now();
    }
}

fn load_save_file(saved: &[u8]) -> Result<Cpu, EmulatorErr> {
    match bincode::decode_from_slice(saved, bincode::config::standard()) {
        Ok((cpu, _)) => Ok(cpu),
        Err(e) => {
            log::error(&format!("Savefile decoding error: {e:?}"));
            Err(EmulatorErr::SaveFileCorrupted)
        }
    }
}
