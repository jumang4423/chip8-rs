use crate::display::Display;
use crate::keypad::Keypad;
use crate::DEBUG_ENABLED;
use minifb::Window;
use rand::Rng;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
pub const FRAME_RATE: u64 = 150; // TODO: ??
const MEMORY_SIZE: usize = 4096;
const STACK_SIZE: usize = 16;
const V_SIZE: usize = 16;
const PROGRAM_START: usize = 0x200;
const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];
const BEEP_PATH: &str = "audio/beep.wav";

enum NiEnum {
    Next,
    Skip,
    Jump(u16),
}

pub struct Cpu {
    pub i: u16,
    pub pc: u16,
    pub memory: [u8; MEMORY_SIZE],
    pub v: [u8; V_SIZE],
    pub keypad: Keypad,
    pub display: Display,
    pub stack: [u16; STACK_SIZE],
    pub sp: u8,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub window: Window,
}

fn read_word(memory: [u8; MEMORY_SIZE], index: u16) -> u16 {
    let first_byte = memory[index as usize] as u16;
    let second_byte = memory[(index + 1) as usize] as u16;

    (first_byte << 8) | second_byte
}

impl Cpu {
    // initialize the cpu
    pub fn new(window: Window) -> Cpu {
        let mut memory = [0; MEMORY_SIZE];
        // load fontset into memory
        for i in 0..80 {
            memory[i] = FONTSET[i];
        }
        Cpu {
            i: 0,
            pc: PROGRAM_START as u16,
            memory: memory,
            v: [0; V_SIZE],
            keypad: Keypad::new(),
            display: Display::new(),
            stack: [0; STACK_SIZE],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            window: window,
        }
    }

    pub fn show_cpu_info(&self) {
        println!("pc: {:X}, i: {:X}, sp: {:X}", self.pc, self.i, self.sp);
        println!(
            "v: {}",
            self.v
                .iter()
                .map(|&v| format!("{:03}", v))
                .collect::<Vec<String>>()
                .join(", ")
        );
        println!(
            "stack: {}",
            self.stack
                .iter()
                .map(|&addr| format!("{:03}", addr))
                .collect::<Vec<String>>()
                .join(", ")
        );
    }

    pub fn load_rom(&mut self, rom_path: &str) {
        let rom = std::fs::read(rom_path).expect("failed to read rom");
        for (i, byte) in rom.iter().enumerate() {
            self.memory[PROGRAM_START + i] = *byte;
        }
    }

    pub fn execute_cycle(&mut self) {
        // run opcode
        let op: u16 = read_word(self.memory, self.pc);
        if *DEBUG_ENABLED.lock().unwrap() {
            println!("excuting opcode: {:X}", op);
        }
        let ni: NiEnum = self.process_opcode(op);
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                self.play_beep();
            }
            self.sound_timer -= 1;
        }
        match ni {
            NiEnum::Next => self.pc += 2,
            NiEnum::Skip => self.pc += 4,
            NiEnum::Jump(addr) => self.pc = addr,
        }
        // update keypad
        self.keypad.update_keys(&mut self.window);

        if *DEBUG_ENABLED.lock().unwrap() {
            self.show_cpu_info();
        }
    }

    fn process_opcode(&mut self, op: u16) -> NiEnum {
        let o1: u8 = ((op & 0xF000) >> 12) as u8;
        let o2: u8 = ((op & 0x0F00) >> 8) as u8;
        let o3: u8 = ((op & 0x00F0) >> 4) as u8;
        let o4: u8 = (op & 0x000F) as u8;
        match (o1, o2, o3, o4) {
            (0x0, 0x0, 0xE, 0x0) => {
                self.display.clear();
                NiEnum::Next
            }
            (0x0, 0x0, 0xE, 0xE) => self.ret(),
            (0x1, _, _, _) => self.addr(o2, o3, o4),
            (0x2, _, _, _) => self.call(o2, o3, o4),
            (0x3, _, _, _) => self.skip_if_equal_byte(o2, o3, o4),
            (0x4, _, _, _) => self.skip_if_not_equal_byte(o2, o3, o4),
            (0x5, _, _, 0x0) => self.skip_if_equal_register(o2, o3),
            (0x6, _, _, _) => self.load_byte(o2, o3, o4),
            (0x7, _, _, _) => self.add_byte(o2, o3, o4),
            (0x8, _, _, 0x0) => self.load_register(o2, o3),
            (0x8, _, _, 0x1) => self.or_register(o2, o3),
            (0x8, _, _, 0x2) => self.and_register(o2, o3),
            (0x8, _, _, 0x3) => self.xor_register(o2, o3),
            (0x8, _, _, 0x4) => self.add_register(o2, o3),
            (0x8, _, _, 0x5) => self.sub_register(o2, o3),
            (0x8, _, _, 0x6) => self.shr_register(o2),
            (0x8, _, _, 0x7) => self.subn_register(o2, o3),
            (0x8, _, _, 0xE) => self.shl_register(o2),
            (0x9, _, _, 0x0) => self.skip_if_not_equal_register(o2, o3),
            (0xA, _, _, _) => self.load_i(o2, o3, o4),
            (0xB, _, _, _) => self.jump_v0(o2, o3, o4),
            (0xC, _, _, _) => self.random_byte(o2, o3, o4),
            (0xD, _, _, _) => self.draw(o2, o3, o4),
            (0xE, _, 0x9, 0xE) => self.skip_if_key_pressed(o2),
            (0xE, _, 0xA, 0x1) => self.skip_if_key_not_pressed(o2),
            (0xF, _, 0x0, 0x7) => self.load_delay_timer(o2),
            (0xF, _, 0x0, 0xA) => self.wait_for_key_press(o2),
            (0xF, _, 0x1, 0x5) => self.set_delay_timer(o2),
            (0xF, _, 0x1, 0x8) => self.set_sound_timer(o2),
            (0xF, _, 0x1, 0xE) => self.add_i(o2),
            (0xF, _, 0x2, 0x9) => self.load_sprite(o2),
            (0xF, _, 0x3, 0x3) => self.load_bcd(o2),
            (0xF, _, 0x5, 0x5) => self.load_registers(o2),
            (0xF, _, 0x6, 0x5) => self.load_registers2(o2),
            _ => panic!("Unknown opcode: {:X}", op),
        }
    }

    fn ret(&mut self) -> NiEnum {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
        NiEnum::Next
    }

    fn addr(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let nnn = ((o2 as u16) << 8) | ((o3 as u16) << 4) | (o4 as u16);
        NiEnum::Jump(nnn)
    }

    fn call(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        self.stack[self.sp as usize] = self.pc;
        let nnn = ((o2 as u16) << 8) | ((o3 as u16) << 4) | (o4 as u16);
        self.sp += 1;
        NiEnum::Jump(nnn)
    }

    fn skip_if_equal_byte(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let x = o2;
        let kk = ((o3 as u16) << 4) | (o4 as u16);
        if self.v[x as usize] == kk as u8 {
            NiEnum::Skip
        } else {
            NiEnum::Next
        }
    }

    fn skip_if_not_equal_byte(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let x = o2;
        let kk = ((o3 as u16) << 4) | (o4 as u16);
        if self.v[x as usize] != kk as u8 {
            NiEnum::Skip
        } else {
            NiEnum::Next
        }
    }

    fn skip_if_equal_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        if self.v[x as usize] == self.v[y as usize] {
            NiEnum::Skip
        } else {
            NiEnum::Next
        }
    }

    fn load_byte(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let x = o2;
        let kk = ((o3 as u16) << 4) | (o4 as u16);
        self.v[x as usize] = kk as u8;
        NiEnum::Next
    }

    fn add_byte(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let x = o2;
        let kk = ((o3 as u16) << 4) | (o4 as u16);
        self.v[x as usize] = self.v[x as usize].wrapping_add(kk as u8);
        NiEnum::Next
    }

    fn load_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        self.v[x as usize] = self.v[y as usize];
        NiEnum::Next
    }

    fn or_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        self.v[x as usize] = self.v[x as usize] | self.v[y as usize];
        NiEnum::Next
    }

    fn and_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        self.v[x as usize] = self.v[x as usize] & self.v[y as usize];
        NiEnum::Next
    }

    fn xor_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        self.v[x as usize] = self.v[x as usize] ^ self.v[y as usize];
        NiEnum::Next
    }

    fn add_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let (result, overflow) = vx.overflowing_add(vy);
        self.v[x as usize] = result;
        self.v[0xF] = overflow as u8;
        NiEnum::Next
    }

    fn sub_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let (result, overflow) = vx.overflowing_sub(vy);
        self.v[x as usize] = result;
        self.v[0xF] = !overflow as u8;
        NiEnum::Next
    }

    fn shr_register(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        let vx = self.v[x as usize];
        self.v[0xF] = vx & 0x1;
        self.v[x as usize] = vx >> 1;
        NiEnum::Next
    }

    fn subn_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let (result, overflow) = vy.overflowing_sub(vx);
        self.v[x as usize] = result;
        self.v[0xF] = !overflow as u8;
        NiEnum::Next
    }

    fn shl_register(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        let vx = self.v[x as usize];
        self.v[0xF] = vx & 0x80;
        self.v[x as usize] = vx << 1;
        NiEnum::Next
    }

    fn skip_if_not_equal_register(&mut self, o2: u8, o3: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        if self.v[x as usize] != self.v[y as usize] {
            NiEnum::Skip
        } else {
            NiEnum::Next
        }
    }

    fn load_i(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let nnn = ((o2 as u16) << 8) | ((o3 as u16) << 4) | (o4 as u16);
        self.i = nnn;
        NiEnum::Next
    }

    fn jump_v0(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let nnn = ((o2 as u16) << 8) | ((o3 as u16) << 4) | (o4 as u16);
        NiEnum::Jump(nnn + self.v[0] as u16)
    }

    fn random_byte(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let x = o2;
        let kk = ((o3 as u16) << 4) | (o4 as u16);
        let random = rand::thread_rng().gen_range(0..255);
        self.v[x as usize] = random & kk as u8;
        NiEnum::Next
    }

    fn draw(&mut self, o2: u8, o3: u8, o4: u8) -> NiEnum {
        let x = o2;
        let y = o3;
        let n = o4;
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let mut sprite = Vec::new();
        for i in 0..n {
            sprite.push(self.memory[(self.i + i as u16) as usize]);
        }
        let collision = self
            .display
            .draw(vx as usize, vy as usize, &sprite, &mut self.window);
        self.v[0xF] = collision as u8;
        NiEnum::Next
    }

    fn skip_if_key_pressed(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        let vx = self.v[x as usize];
        if self.keypad.is_key_pressed(vx) {
            NiEnum::Skip
        } else {
            NiEnum::Next
        }
    }

    fn skip_if_key_not_pressed(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        let vx = self.v[x as usize];
        if !self.keypad.is_key_pressed(vx) {
            NiEnum::Skip
        } else {
            NiEnum::Next
        }
    }

    fn load_delay_timer(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        self.v[x as usize] = self.delay_timer;
        NiEnum::Next
    }

    fn wait_for_key_press(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        self.keypad.wait_for_key_press(&mut self.v[x as usize]);
        NiEnum::Next
    }

    fn set_delay_timer(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        self.delay_timer = self.v[x as usize];
        NiEnum::Next
    }

    fn set_sound_timer(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        self.sound_timer = self.v[x as usize];
        NiEnum::Next
    }

    fn add_i(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        self.i += self.v[x as usize] as u16;
        NiEnum::Next
    }

    fn load_sprite(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        let vx = self.v[x as usize];
        self.i = vx as u16 * 5;
        NiEnum::Next
    }

    fn load_bcd(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        let vx = self.v[x as usize];
        let hundreds = vx / 100;
        let tens = (vx % 100) / 10;
        let ones = vx % 10;
        self.memory[self.i as usize] = hundreds;
        self.memory[(self.i + 1) as usize] = tens;
        self.memory[(self.i + 2) as usize] = ones;
        NiEnum::Next
    }

    fn load_registers(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        for i in 0..x + 1 {
            self.memory[(self.i + i as u16) as usize] = self.v[i as usize];
        }
        NiEnum::Next
    }

    fn load_registers2(&mut self, o2: u8) -> NiEnum {
        let x = o2;
        for i in 0..x + 1 {
            self.v[i as usize] = self.memory[(self.i + i as u16) as usize];
        }
        NiEnum::Next
    }

    fn play_beep(&mut self) {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let file = BufReader::new(File::open(BEEP_PATH).unwrap());
        let source = Decoder::new(file).unwrap();
        sink.append(source);
        sink.sleep_until_end();
    }
}
