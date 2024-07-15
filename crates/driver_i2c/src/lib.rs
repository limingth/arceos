#![no_std]

use core::fmt::Write;
use ssd1306::{
    prelude::*, 
    I2CDisplayInterface, 
    Ssd1306,
    mode::BufferedGraphicsMode,
    mode::BasicMode,
};



pub fn run_iicoled() {
	let mut oled_sda = io.pins.gpio21.into_push_pull_output();
    let mut oled_scl = io.pins.gpio22.into_push_pull_output();
	let mut i2c = BlockingI2c::i2c1(
		peripherals.I2C0,
        oled_sda,
        oled_scl,
        100u32.kHz(),
        &clocks,
	);
    let interface = I2CDisplayInterface::new(i2c);

    let mut display =
        Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0).into_terminal_mode();
    display.init().unwrap();
    display.clear().unwrap();

    // Spam some characters to the display
    for c in 97..123 {
        let _ = display.write_str(unsafe { core::str::from_utf8_unchecked(&[c]) });
    }
    for c in 65..91 {
        let _ = display.write_str(unsafe { core::str::from_utf8_unchecked(&[c]) });
    }

    // The `write!()` macro is also supported
    write!(display, "Hello, {}", "world");
}
