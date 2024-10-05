#[cfg(feature = "buddy-alloc")]
mod alloc;
mod wasm4;
mod wasm4_mmio;
use wasm4::{blit, text, BLIT_1BPP, BUTTON_1, BUTTON_DOWN, BUTTON_LEFT, BUTTON_RIGHT, BUTTON_UP};
use wasm4_mmio::{DRAW_COLORS, GAMEPAD1, PALETTE};

// Palettes from https://itch.io/jam/gbpixelartjam24
const PALETTE_BLOOD_TIDE: [u32; 4] = [0x652121, 0x394a5a, 0x7a968d, 0xfffeea];
const PALETTE_FORGOTTEN_SWAMP: [u32; 4] = [0x3b252e, 0x593a5f, 0x4d7d65, 0xd1ada1];
const PALETTE_HOMEWORK: [u32; 4] = [0x12121b, 0x45568d, 0x878c9d, 0xe1d8d4];
const PALETTE_MANGAVANIA: [u32; 4] = [0x6e1a4b, 0xe64ca4, 0x4aedff, 0xffffff];

#[rustfmt::skip]
const SMILEY: [u8; 8] = [
    0b11000011,
    0b10000001,
    0b00100100,
    0b00100100,
    0b00000000,
    0b00100100,
    0b10011001,
    0b11000011,
];

#[no_mangle]
fn start() {
    PALETTE.write(PALETTE_BLOOD_TIDE);
}

#[no_mangle]
fn update() {
    DRAW_COLORS.write(0x0002);
    text("Hello from Rust!", 10, 10);

    let gamepad = GAMEPAD1.read();
    if gamepad & BUTTON_UP != 0 {
        PALETTE.write(PALETTE_BLOOD_TIDE);
    }
    if gamepad & BUTTON_LEFT != 0 {
        PALETTE.write(PALETTE_FORGOTTEN_SWAMP);
    }
    if gamepad & BUTTON_DOWN != 0 {
        PALETTE.write(PALETTE_HOMEWORK);
    }
    if gamepad & BUTTON_RIGHT != 0 {
        PALETTE.write(PALETTE_MANGAVANIA);
    }
    if gamepad & BUTTON_1 != 0 {
        DRAW_COLORS.write(0x0004);
    }

    blit(&SMILEY, 76, 76, 8, 8, BLIT_1BPP);
    text("Press X to blink", 16, 90);
}
