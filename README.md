GBemu
=====

A gameboy emulator written in the Rust programming language.
It does not aim for perfect accuracy but can run most ROMs just fine.

It passes Blargg's `cpu_instr.gb` and `instr_timing.gb` test ROMs' tests.

Features:

- Audio support (APU implementation)
- Mostly accurate rendering
- Color palettes for monochrome games
- MBC1 implementation (others to be implemented)

Screenshots
-----------

![gbemu running the Tetris ROM using a color palette](tetris_gbrom.png)

Keybindings
-----------

| Gameboy Key | Keyboard Key |
| ----------- | ------------ |
| START       | backspace    |
| SELECT      | return       |
| A           | Z            |
| B           | X            |
| UP          | W/↑          |
| DOWN        | S/↓          |
| LEFT        | A/←          |
| RIGHT       | D/→          |

| Function       | Keyboard Key |
| -------------- | ------------ |
| Change palette | space        |
| Exit emulator  | escape       |

Building
--------

It depends on [SDL3](https://wiki.libsdl.org/SDL3/FrontPage) for graphics and
sound, so install it first before building.  
SDL3 can be installed by following the instructions here
[SDL3 Installation](https://github.com/libsdl-org/SDL/blob/main/INSTALL.md).
Some platforms provide it via a package manager, I have listed a few common ones below:

- MacOS: `brew install sdl3`
- Fedora: `dnf install SDL3-devel`
- Ubuntu: `apt install libsdl3-dev`

After installing SDL3 build using cargo:

```bash
cargo build --release
```

Run using cargo:

```bash
cargo run --release -- <ROM-path>
```

TODOs
-----

- Implement remaining MBCs
- Implement CGB mode
