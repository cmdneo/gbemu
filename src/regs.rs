//! IO-port register structures for conveninet reading and writing.
use serde::{Deserialize, Serialize};

use crate::macros::bit_fields;

bit_fields! {
    /// Joypad/P1 register, only upper nibble is writable by user-code.
    /// In this register, rather unconventionally 0-bit means PRESSED,
    /// so complement bits before writng to the actual register.
    ///
    /// Lower 4-bits are set as: `ActionButtons` for `select_buttons`
    /// and `Dpad` for `select_dpad`.
    #[derive(Deserialize, Serialize)]
    pub(crate) struct JoyPad<u8> {
        state: 4,
        select_dpad: 1,
        select_buttons: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct DPad<u8> {
        right: 1,
        left: 1,
        up: 1,
        down: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct ActionButtons<u8> {
        a: 1,
        b: 1,
        select: 1,
        start: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct SerialCtrl<u8> {
        clock_select: 1,
        clock_speed: 1,
        _0: 5,
        tx_enable: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct LcdCtrl<u8> {
        /// In non-CGB mode this overrides win_enable
        /// and has meaning `BG_and_window_enable`.
        bg_win_priotity: 1,
        obj_enable: 1,
        obj_size: 1,
        bg_tile_map: 1,
        /// Addressing mode for BG/Win tile index in tile data.
        bg_win_tile_data: 1,
        win_enable: 1,
        win_tile_map: 1,
        ppu_enable: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct LcdStat<u8> {
        ppu_mode: 2,
        ly_eq_lyc: 1,
        // Conditions for STAT interrupt.
        mode0: 1,
        mode1: 1,
        mode2: 1,
        lyc_int: 1,
    }
}

bit_fields! {
    /// Background/Object color palette index.
    #[derive(Deserialize, Serialize)]
    pub(crate) struct CgbPaletteIndex<u8> {
        addr: 6,
        _0: 1,
        auto_inc: 1,
    }
}

bit_fields! {
    /// Background/Object color palette index.
    #[derive(Deserialize, Serialize)]
    pub(crate) struct CgbColor<u16> {
        red: 5,
        green: 5,
        blue: 5,
    }
}

bit_fields! {
    /// `TIMA` register control information.
    #[derive(Deserialize, Serialize)]
    pub(crate) struct TimerCtrl<u8> {
        clock_select: 2,
        enable: 1,
    }
}

bit_fields! {
    /// Interrupt data, IE and IF register fields.
    #[derive(Deserialize, Serialize)]
    pub(crate) struct IntrBits<u8> {
        vblank: 1,
        stat: 1,
        timer: 1,
        serial: 1,
        joypad: 1,
    }
}

impl IntrBits {
    pub(crate) fn masked(self, mask: Self) -> Self {
        Self {
            vblank: self.vblank & mask.vblank,
            stat: self.stat & mask.stat,
            timer: self.timer & mask.timer,
            serial: self.serial & mask.serial,
            joypad: self.joypad & mask.joypad,
        }
    }
}

bit_fields! {
    /// Dual-speed(for CGB) speed switch register(KEY1).
    #[derive(Deserialize, Serialize)]
    pub(crate) struct Key1<u8> {
        armed: 1,
        _1: 6,
        speed: 1,
    }
}

// Audio control registers
// Audio registers which do not follow the NRxy convenction
// have their specialized type.
// --------------------------------------------------------

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioNr52<u8> {
        ch1_on: 1,
        ch2_on: 1,
        ch3_on: 1,
        ch4_on: 1,
        _0: 3,
        enable: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioNr51<u8> {
        ch1_right: 1,
        ch2_right: 1,
        ch3_right: 1,
        ch4_right: 1,
        ch1_left: 1,
        ch2_left: 1,
        ch3_left: 1,
        ch4_left: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioNr50<u8> {
        vol_right: 3,
        vin_right: 1,
        vol_left: 3,
        vin_left: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioNx0<u8> {
        shift_step: 3,
        direction: 1,
        pace: 3,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioNx1<u8> {
        length_period: 6,
        wave_duty: 2,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioNx2<u8> {
        pace: 3,
        direction: 2,
        initial_volume: 4,
    }
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct AudioNx3 {
    pub(crate) period_low: u8,
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioNx4<u8> {
        period_high: 3,
        _0: 3,
        length_timer_enable: 1,
        trigger: 1,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioN30<u8> {
        _0: 7,
        dac_on: 1,
    }
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct AudioN31 {
    pub(crate) length_period: u8,
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioN32<u8> {
        _0: 5,
        output_level: 2,
    }
}

bit_fields! {
    #[derive(Deserialize, Serialize)]
    pub(crate) struct AudioN43<u8> {
        clock_divider: 3,
        lfsr_width: 1,
        clock_shift: 4,
    }
}
