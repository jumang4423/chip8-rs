use minifb::Window;
pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;

pub struct Display {
    pub pixels: [[u8; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
}

impl Display {
    pub fn new() -> Display {
        Display {
            pixels: [[0; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
        }
    }

    pub fn clear(&mut self) {
        self.pixels = [[0; DISPLAY_WIDTH]; DISPLAY_HEIGHT];
    }

    pub fn draw(&mut self, x: usize, y: usize, sprite: &Vec<u8>, window: &mut Window) -> bool {
        let mut collision = false;
        let mut is_updated = false;
        for (i, line) in sprite.iter().enumerate() {
            for j in 0..8 {
                let pixel = (line >> (7 - j)) & 0x1;
                let x_pos = (x + j) % DISPLAY_WIDTH;
                let y_pos = (y + i) % DISPLAY_HEIGHT;
                if pixel == 1 {
                    is_updated = true;
                    if self.pixels[y_pos][x_pos] == 1 {
                        collision = true;
                    }
                    self.pixels[y_pos][x_pos] ^= pixel;
                }
            }
        }
        if is_updated {
            self.update_to_display(window);
        }
        collision
    }

    fn update_to_display(&mut self, window: &mut Window) {
        window
            .update_with_buffer(&self.pixels_to_buffer(), DISPLAY_WIDTH, DISPLAY_HEIGHT)
            .unwrap();
    }

    fn pixels_to_buffer(&self) -> [u32; DISPLAY_WIDTH * DISPLAY_HEIGHT] {
        let mut buffer = [0; DISPLAY_WIDTH * DISPLAY_HEIGHT];
        for (i, row) in self.pixels.iter().enumerate() {
            for (j, pixel) in row.iter().enumerate() {
                buffer[i * DISPLAY_WIDTH + j] = if *pixel == 1 { 0xFFFFFF } else { 0x000000 };
            }
        }
        buffer
    }
}
