use std::{
    sync::mpsc::{Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use crate::{
    cartridge::Cartidge,
    cpu::Cpu,
    mmu::Mmu,
    msg::{Reply, Request, VideoFrame},
    EmulatorErr,
};

/// Number of CPU steps to run in one go.
// Max steps run at once must be less than VBLANK interval,
// because we capture a frame for rendering only in VBLANK.
// VBLANK is 4560 dots and the longest it takes for a step is 24 dots.
// So number of steps should be always less than 190 (=4560/24).
const STEPS_PER_BURST: usize = 150;

pub struct Emulator {
    cpu: Cpu,
    /// Total T-cycles ticked since last `timer_reset`.
    tcycles: u64,
    /// Time when the timer was reset.
    start_time: Instant,
    /// Actual clock frequency achieved by the emulator
    real_frequency: f64,
    is_running: bool,
}

impl Emulator {
    pub fn new(rom: Vec<u8>) -> Result<Self, EmulatorErr> {
        let cartidge = Cartidge::new(rom)?;
        let mmu = Mmu::new(cartidge);
        let cpu = Cpu::new(mmu);

        Ok(Self {
            cpu,
            tcycles: 0,
            real_frequency: 0.0,
            start_time: Instant::now(),
            is_running: false,
        })
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

        self.init();
        self.reset_timers();
        self.is_running = true;
        // self.cpu.trace_execution = true;

        while self.is_running {
            for _ in 0..STEPS_PER_BURST {
                self.step();
            }

            assert!(self.handle_audio_flow(&audio_ctrl_rx, &audio_data_tx));
            assert!(self.handle_msgs(&request_rx, &reply_tx));
            self.manage_sleep_timer();
        }
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
    /// Returns false if send/recieve failed, otherwise true.
    fn handle_msgs(&mut self, request_rx: &Receiver<Request>, reply_tx: &Sender<Reply>) -> bool {
        let msg = match request_rx.try_recv() {
            Ok(msg) => msg,
            Err(TryRecvError::Empty) => return true,
            Err(TryRecvError::Disconnected) => return false,
        };

        match msg {
            Request::Start => panic!("already running"),

            Request::UpdateButtonState(btns) => {
                let (dpad, btns) = btns.to_internal_repr();
                self.cpu.mmu.update_joypad(dpad, btns);
                true
            }

            Request::CyclePalette => {
                self.cpu.mmu.ppu.cycle_palette(1);
                true
            }

            Request::GetVideoFrame => {
                let mut f = Box::new(VideoFrame::default());
                self.cpu.mmu.ppu.copy_frame(f.as_mut());
                reply_tx.send(Reply::VideoFrame(f)).is_ok()
            }

            Request::GetTitle => reply_tx
                .send(Reply::Title(self.cpu.mmu.cart.title.clone()))
                .is_ok(),

            Request::GetFrequency => reply_tx.send(Reply::Frequency(self.real_frequency)).is_ok(),

            Request::Shutdown => {
                self.is_running = false;
                reply_tx.send(Reply::ShuttingDown).is_ok()
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
    ) -> bool {
        match audio_ctrl_rx.try_recv() {
            Ok(period) => audio_data_tx
                .send(
                    self.cpu
                        .mmu
                        .apu
                        .start_new_sampling(period)
                        .into_boxed_slice(),
                )
                .is_ok(),
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => false,
        }
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
