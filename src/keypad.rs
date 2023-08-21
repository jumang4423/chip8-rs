use minifb::Window;

const KEY_MAP: [(char, minifb::Key, u8); 16] = [
    ('1', minifb::Key::Key1, 0x1),
    ('2', minifb::Key::Key2, 0x2),
    ('3', minifb::Key::Key3, 0x3),
    ('4', minifb::Key::Key4, 0xC),
    ('q', minifb::Key::Q, 0x4),
    ('w', minifb::Key::W, 0x5),
    ('e', minifb::Key::E, 0x6),
    ('r', minifb::Key::R, 0xD),
    ('a', minifb::Key::A, 0x7),
    ('s', minifb::Key::S, 0x8),
    ('d', minifb::Key::D, 0x9),
    ('f', minifb::Key::F, 0xE),
    ('z', minifb::Key::Z, 0xA),
    ('x', minifb::Key::X, 0x0),
    ('c', minifb::Key::C, 0xB),
    ('v', minifb::Key::V, 0xF),
];

pub struct Keypad {
    keys: [bool; 16],
}

impl Keypad {
    pub fn new() -> Keypad {
        Keypad { keys: [false; 16] }
    }

    pub fn update_keys(&mut self, window: &mut Window) {
        for key in KEY_MAP.iter() {
            let is_pressed = window.is_key_down(key.1);
            self.keys[key.2 as usize] = is_pressed;
        }
    }

    pub fn is_key_pressed(&self, key: u8) -> bool {
        self.keys[key as usize]
    }

    pub fn wait_for_key_press(&mut self, vx: &mut u8) {
        loop {
            for (i, key) in self.keys.iter().enumerate() {
                if *key {
                    *vx = i as u8;
                    return;
                }
            }
        }
    }
}
