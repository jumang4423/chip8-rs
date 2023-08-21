mod cpu;
mod display;
mod keypad;
use cpu::{Cpu, FRAME_RATE};
use lazy_static::lazy_static;
use minifb::{Scale, Window, WindowOptions};
use std::env;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

lazy_static! {
    pub static ref DEBUG_ENABLED: Mutex<bool> = Mutex::new(true);
}

fn main() {
    // get rom path from args
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Usage: cargo run <rom_path>");
    }
    run_rom(&args[1]);
}

fn run_rom(rom_path: &str) {
    // create cpu
    let window = Window::new(
        "Chip-8 Emulator",
        display::DISPLAY_WIDTH,
        display::DISPLAY_HEIGHT,
        WindowOptions {
            scale: Scale::X16,
            ..WindowOptions::default()
        },
    )
    .unwrap();
    let mut cpu = Cpu::new(window);
    cpu.load_rom(rom_path);
    // start cpu
    let frame_duration = Duration::from_millis(1000 / FRAME_RATE);
    let mut last_time = Instant::now();
    loop {
        cpu.execute_cycle();
        let elapsed = last_time.elapsed();
        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
        last_time = Instant::now();
    }
}
