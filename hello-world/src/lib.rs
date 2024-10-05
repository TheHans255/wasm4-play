#[cfg(feature = "buddy-alloc")]
mod alloc;
mod wasm4;
mod wasm4_mmio;
use wasm4::{blit, text, BLIT_1BPP, BUTTON_1};
use wasm4_mmio::{DRAW_COLORS, GAMEPAD1, PALETTE};

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
    PALETTE.write([0xfffeea, 0x7a968d, 0x394a5a, 0x652121])
}

#[no_mangle]
fn update() {
    DRAW_COLORS.write(0x0002);
    text("Hello from Rust!", 10, 10);

    let gamepad = GAMEPAD1.read();
    if gamepad & BUTTON_1 != 0 {
        DRAW_COLORS.write(0x0004);
    }

    blit(&SMILEY, 76, 76, 8, 8, BLIT_1BPP);
    text("Press X to blink", 16, 90);
}
