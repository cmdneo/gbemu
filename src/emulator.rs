use std::{
    sync::mpsc::{self, RecvError, TryRecvError},
    time::Instant,
};

use macroquad::{
    miniquad::date::now,
    rand::{rand, srand},
};

use crate::{
    cartridge::Cartidge,
    cpu::{Cpu, CpuState},
    frame::Frame,
    info,
    mem::Mmu,
    msg::{EmulatorMsg, UserMsg},
    EmuError,
};

/// Number of CPU steps to run in one go.
// Max steps run at once must be less than VBLANK interval,
// because we capture a frame for rendering only in VBLANK.
// VBLANK is 4560 dots and the longest it takes for a step is 24 dots.
// Why 24 dots? It takes max 6 mcycles for an instruction and each
// mcycle is made up of 2 or 4 dots, and 4*6 = 24.
// So number of steps should be always less than 190 (=4560/24).
const STEPS_PER_BURST: usize = 150;

pub struct Emulator {
    cpu: Cpu,
    /// Total T-cycles ticked since last `timer_reset`.
    tcycles: u64,
    /// Time when the timer was reset.
    start_time: Instant,
    target_freq: u32,
    actual_freq: f64,
    is_running: bool,
    frame_requested: bool,
}

impl Emulator {
    pub fn new(rom: &[u8]) -> Result<Self, EmuError> {
        let cartidge = Cartidge::new(rom)?;
        let mmu = Mmu::new(cartidge);
        let cpu = Cpu::new(mmu);

        Ok(Self {
            cpu,
            tcycles: 0,
            target_freq: info::FREQUENCY,
            actual_freq: 0.0,
            start_time: Instant::now(),
            is_running: false,
            frame_requested: false,
        })
    }

    /// Start the emulator and run until `UserMsg::Shutdown` is recieved.
    /// Run it in a new thread and use channels to communicate with
    /// it: buttons presses, frame requests and other commands.
    ///
    /// Parameters:  
    /// `user_msg_rx`: For recieving messages for controlling the emulator.  
    /// `emu_msg_tx` : For sending replies(if any) for recieved messages.
    pub fn run(
        &mut self,
        user_msg_rx: mpsc::Receiver<UserMsg>,
        emu_msg_tx: mpsc::Sender<EmulatorMsg>,
    ) {
        self.init();
        self.reset_timers();
        self.is_running = true;
        // self.cpu.trace_execution = true;

        while self.is_running {
            for _ in 0..STEPS_PER_BURST {
                self.step();
            }

            // If CPU is stopped then we wait in blocking mode.
            if !self.handle_msgs(
                &user_msg_rx,
                &emu_msg_tx,
                self.cpu.state != CpuState::Stopped,
            ) {
                panic!("send/recieve channels closed abnormally");
            }

            // Only send back frame after entring VBLANK mode to avoid screen jitter.
            if self.frame_requested && self.cpu.mmu.get_mode() == info::MODE_VBLANK {
                let mut f = Box::new(Frame::default());
                self.cpu.mmu.ppu.copy_frame(f.as_mut());
                self.frame_requested = false;
                emu_msg_tx.send(EmulatorMsg::NewFrame(f)).unwrap();
            }

            // Busy-wait until clock starts lagging behind.
            loop {
                let elapsed = self.start_time.elapsed().as_secs_f64();
                let expected = elapsed * self.target_freq as f64;
                let actual = self.tcycles as f64;

                if expected > actual {
                    self.actual_freq = actual / elapsed;
                    break;
                }
            }
        }
    }

    /// Run a for a step each component.
    // Runs each component step-by-step.
    // In the real hardware eveything is synchronized by a master clock.
    // Here, we try to achieve the same effect by running each component
    // step-by-step. It is as if the CPU produces cycles and other components
    // (PPU and Timer) consume it.
    //
    // First we run the CPU and check how many cycles it used,
    // then run other components for exactly than many cycles.
    // This simplifies synchronization and timings.
    fn step(&mut self) {
        let was_2x = self.cpu.mmu.is_2x;
        let mcycles = self.cpu.step();

        if self.cpu.state == CpuState::Stopped {
            self.reset_timers();
            return;
        }

        // Adjust frequency after speed switch.
        if !was_2x && self.cpu.mmu.is_2x {
            self.reset_timers();
            self.target_freq = info::FREQUENCY_2X;
        }

        self.tcycles += mcycles as u64 * 4;
    }

    /// Handle user messages and respond to them.
    /// Returns false if send/recieve failed, otherwise true.
    fn handle_msgs(
        &mut self,
        msg_rx: &mpsc::Receiver<UserMsg>,
        msg_tx: &mpsc::Sender<EmulatorMsg>,
        non_blocking: bool,
    ) -> bool {
        let msg = if non_blocking {
            match msg_rx.try_recv() {
                Ok(msg) => msg,
                Err(TryRecvError::Empty) => return true,
                Err(TryRecvError::Disconnected) => return false,
            }
        } else {
            match msg_rx.recv() {
                Ok(msg) => msg,
                Err(RecvError) => return false,
            }
        };

        match msg {
            UserMsg::UpdateButtons(btns) => {
                let (dpad, btns) = btns.to_internal_repr();
                self.cpu.mmu.update_joypad(dpad, btns);
                true
            }

            UserMsg::CyclePalette => {
                todo!()
            }

            UserMsg::GetFrame => {
                // Send frame only on VBLANK to avoid choppiness.
                self.frame_requested = true;
                true
            }

            UserMsg::GetFrequency => msg_tx
                .send(EmulatorMsg::Frequency(self.actual_freq))
                .is_ok(),

            UserMsg::Shutdown => {
                self.is_running = false;
                self.cpu.mmu.apu.stop_audio();
                msg_tx.send(EmulatorMsg::ShuttingDown).is_ok()
            }

            UserMsg::ClearFrame(c) => {
                self.cpu.mmu.ppu.paint_frame(c);
                true
            }

            UserMsg::DebuggerStart => todo!(),
            UserMsg::DebuggerStep => todo!(),
            UserMsg::DebuggerStop => todo!(),
        }
    }

    /// Initialize the registers and state, make it ready for execution.
    fn init(&mut self) {
        // Initial values for starting up the program.
        self.cpu.pc.0 = 0x0100;
        self.cpu.sp.0 = 0xFFFE;

        // TODO Use a bootloader for initialization stuff
        let m = &mut self.cpu.mmu;

        m.joypad.write(0xCF);
        m.wram_idx = 1;
        m.ppu.bgp = 0xFC;
        m.ppu.fetcher.lcdc.write(0x91);
        m.ppu.stat.write(0x85);

        srand((now() * 1000.0) as u64);
        for n in m.ppu.bg_palette.iter_mut() {
            *n = rand() as u8;
        }
        for n in m.ppu.bg_palette.iter_mut() {
            *n = rand() as u8;
        }

        self.cpu.mmu.apu.play_audio();
    }

    fn reset_timers(&mut self) {
        self.tcycles = 0;
        self.start_time = Instant::now();
    }
}
