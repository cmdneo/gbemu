use std::{
    sync::mpsc::{self, RecvError, TryRecvError},
    time::Instant,
};

use crate::{
    cartridge::Cartidge,
    cpu::Cpu,
    display::Frame,
    info, log,
    mem::Mmu,
    msg::{EmulatorMsg, UserMsg},
    ppu::Ppu,
    timer::Timer,
    EmuError,
};

pub struct Emulator {
    cpu: Cpu,
    ppu: Ppu,
    timer: Timer,
    /// Total T-cycles ticked since last `timer_reset`.
    tcycles: u64,
    target_freq: u32,
    actual_freq: f64,
    start_time: Instant,
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
            ppu: Ppu::new(),
            timer: Timer::new(),
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
            self.step();

            // If CPU is stopped then we wait in blocking mode.
            if !self.handle_msgs(&user_msg_rx, &emu_msg_tx, !self.cpu.is_stopped) {
                log::error("emulator: send/recieve channels closed abnormally");
                break;
            }

            if self.frame_requested && self.cpu.mmu.get_mode() == info::MODE_VBLANK {
                let mut f = Box::new(Frame::default());

                self.ppu.fill_frame(f.as_mut());
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

    pub fn reset_timers(&mut self) {
        self.tcycles = 0;
        self.start_time = Instant::now();
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
        let mcycles = self.cpu.step();
        if self.cpu.is_stopped {
            return;
        }

        // Dual-speed mode does not change PPU or Audio speed.
        let dots = mcycles * if self.cpu.mmu.is_cgb { 2 } else { 4 };

        // On speed-switch DIV clock does not tick, audio and video are not
        // processed properly for some time.
        // So, on speed-switch we reset PPU and Audio processes as those may
        // cause weird interrupts and audio/visual jitter.
        if mcycles >= info::SPEED_SWITCH_MCYCLES {
            self.reset_timers();
            self.target_freq = info::FREQUENCY_2X;
        } else {
            self.timer.step(&mut self.cpu.mmu, mcycles);
            self.ppu.tick(&mut self.cpu.mmu, dots);
            // self.audio.step(&mut self.cpu.mem, norm_tcycles);
            self.tcycles += mcycles as u64 * 4;
        }

        self.cpu.mmu.step(mcycles);
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
            UserMsg::Buttons(btns) => {
                let (dpad, btns) = btns.to_internal_repr();
                self.cpu.mmu.update_joypad(dpad, btns);
                true
            }

            UserMsg::GetFrame => {
                // Send frame only on VBLANK to avoid jitter.
                self.frame_requested = true;
                true
            }

            UserMsg::GetFrequency => msg_tx
                .send(EmulatorMsg::Frequency(self.actual_freq))
                .is_ok(),

            UserMsg::Shutdown => {
                self.is_running = false;
                msg_tx.send(EmulatorMsg::ShuttingDown).is_ok()
            }

            UserMsg::ClearFrame(_) => todo!(),
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

        self.cpu.mmu.set_reg(info::IO_JOYPAD, 0x3F);
        self.cpu.mmu.set_reg(info::IO_SVBK, 1);
        self.cpu.mmu.set_reg(info::IO_BGP, 0xFC);
        self.cpu.mmu.set_reg(info::IO_LCDC, 0x91);
        self.cpu.mmu.set_reg(info::IO_STAT, 0x85);
    }
}
