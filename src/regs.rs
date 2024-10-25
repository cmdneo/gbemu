//! IO-port register structures for conveninet reading and writing.

use crate::macros::bit_fields;

bit_fields! {
    /// Joypad/P1 register, only upper nibble is writable by user-code.
    /// In this register, rather unconventionally 0-bit means PRESSED,
    /// so complement bits before writng to the actual register.
    ///
    /// Lower 4-bits are set as: `ActionButtons` for `select_buttons`
    /// and `Dpad` for `select_dpad`.
    pub(crate) struct JoyPad<u8> {
        state: 4,
        select_dpad: 1,
        select_buttons: 1,
    }
}

bit_fields! {
    pub(crate) struct DPad<u8> {
        right: 1,
        left: 1,
        up: 1,
        down: 1,
    }
}

bit_fields! {
    pub(crate) struct ActionButtons<u8> {
        a: 1,
        b: 1,
        select: 1,
        start: 1,
    }
}

bit_fields! {
    pub(crate) struct SerialCtrl<u8> {
        clock_select: 1,
        clock_speed: 1,
        _0: 5,
        tx_enable: 1,
    }
}

bit_fields! {
    #[derive(Debug)]
    pub(crate) struct LcdCtrl<u8> {
        /// In non-CGB mode this overrides win_enable
        /// and has meaning `BG_and_window_enable`.
        bg_win_priotity:1,
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
    #[derive(Debug)]
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
    pub(crate) struct CgbPaletteIndex<u8> {
        addr: 6,
        _0: 1,
        auto_inc: 1,
    }
}

bit_fields! {
    /// Background/Object color palette index.
    pub(crate) struct CgbPaletteColor<u16> {
        red: 5,
        green: 5,
        blue: 5,
    }
}

bit_fields! {
    /// `TIMA` register control information.
    pub(crate) struct TimerCtrl<u8> {
        clock_select: 2,
        enable: 1,
    }
}

bit_fields! {
    /// Interrupt data, IE and IF register fields.
    pub(crate) struct IntrBits<u8> {
        vblank: 1,
        stat: 1,
        timer: 1,
        serial: 1,
        joypad: 1,
    }
}

bit_fields! {
    /// Dual-speed(for CGB) speed switch register(KEY1).
    pub(crate) struct Key1<u8> {
        armed: 1,
        _1: 6,
        speed: 1,
    }
}

bit_fields! {
    pub(crate) struct AudioCtrl<u8> {
        ch1: 1,
        ch2: 1,
        ch3: 1,
        ch4: 1,
        _0: 3,
        enable: 1,
    }
}

bit_fields! {
    pub(crate) struct AudioPaning<u8> {
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
    pub(crate) struct AudioVinNVol<u8> {
        vol_right: 3,
        vin_right: 1,
        vol_left: 3,
        vin_left: 1,
    }
}

bit_fields! {
    pub(crate) struct Rp<u8> {
        on: 1,
        fixed: 1,
        _1: 4,
        read_en: 2,
    }
}
