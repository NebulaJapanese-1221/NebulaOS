use super::gui::{Application, Button, Canvas, Event, PointerEvent, Rect};

const KEY_VALUES: [u8; 16] = [
    b'7', b'8', b'9', b'/', b'4', b'5', b'6', b'*', b'1', b'2', b'3', b'-', b'C', b'0', b'=',
    b'+',
];

pub struct Calculator {
    input: [u8; 64],
    input_len: usize,
    result: [u8; 32],
    result_len: usize,
    buttons: [Button; 16],
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            input: [0; 64],
            input_len: 0,
            result: [0; 32],
            result_len: 0,
            buttons: [
                Button::new("7"), Button::new("8"), Button::new("9"), Button::new("/"),
                Button::new("4"), Button::new("5"), Button::new("6"), Button::new("*"),
                Button::new("1"), Button::new("2"), Button::new("3"), Button::new("-"),
                Button::new("C"), Button::new("0"), Button::new("="), Button::new("+"),
            ],
        }
    }

    fn press_key(&mut self, key: u8) -> bool {
        match key {
            b'c' | b'C' => {
                self.input_len = 0;
                self.result_len = 0;
                true
            }
            b'=' | b'\n' => {
                self.evaluate();
                true
            }
            8 if self.input_len > 0 => {
                self.input_len -= 1;
                self.result_len = 0;
                true
            }
            b'0'..=b'9' | b'+' | b'-' | b'*' | b'/' if self.input_len < self.input.len() => {
                self.input[self.input_len] = key;
                self.input_len += 1;
                self.result_len = 0;
                true
            }
            _ => false,
        }
    }

    fn evaluate(&mut self) {
        let expression = &self.input[..self.input_len];
        let Some(operator_index) = expression
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, byte)| matches!(byte, b'+' | b'-' | b'*' | b'/').then_some(index))
        else {
            self.set_result(b"INVALID");
            return;
        };
        let (Ok(left), Ok(right)) = (
            parse_integer(&expression[..operator_index]),
            parse_integer(&expression[operator_index + 1..]),
        ) else {
            self.set_result(b"INVALID");
            return;
        };
        let value = match expression[operator_index] {
            b'+' => left.checked_add(right),
            b'-' => left.checked_sub(right),
            b'*' => left.checked_mul(right),
            b'/' if right != 0 => left.checked_div(right),
            b'/' => {
                self.set_result(b"DIV BY ZERO");
                return;
            }
            _ => None,
        };
        if let Some(value) = value {
            self.set_integer_result(value);
        } else {
            self.set_result(b"OUT OF RANGE");
        }
    }

    fn set_result(&mut self, result: &[u8]) {
        self.result = [0; 32];
        self.result_len = result.len().min(self.result.len());
        self.result[..self.result_len].copy_from_slice(&result[..self.result_len]);
    }

    fn set_integer_result(&mut self, value: i32) {
        let negative = value < 0;
        let mut magnitude = value.unsigned_abs();
        let mut reversed = [0u8; 11];
        let mut length = 0;
        loop {
            reversed[length] = b'0' + (magnitude % 10) as u8;
            length += 1;
            magnitude /= 10;
            if magnitude == 0 {
                break;
            }
        }
        self.result = [0; 32];
        self.result_len = length + usize::from(negative);
        let mut output_index = 0;
        if negative {
            self.result[0] = b'-';
            output_index = 1;
        }
        for index in (0..length).rev() {
            self.result[output_index] = reversed[index];
            output_index += 1;
        }
    }

    fn button_rect(area: Rect, index: usize) -> Rect {
        let gap = 6;
        let column = (index % 4) as u32;
        let row = (index / 4) as u32;
        let button_width = area.width.saturating_sub(44 + gap * 3) / 4;
        Rect {
            x: area.x + 22 + column * (button_width + gap),
            y: area.y + 82 + row * 37,
            width: button_width,
            height: 31,
        }
    }
}

impl Application for Calculator {
    fn title(&self) -> &'static str {
        "CALCULATOR"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        let output = Rect {
            x: area.x + 22,
            y: area.y + 20,
            width: area.width.saturating_sub(44),
            height: 52,
        };
        canvas.fill_rect(output, 0x172236);
        if let Ok(input) = core::str::from_utf8(&self.input[..self.input_len]) {
            canvas.draw_text(output.x + 10, output.y + 9, input, 0xF1F5F9, 1);
        }
        let result = if self.result_len == 0 {
            "TYPE EXPRESSION, THEN ="
        } else {
            core::str::from_utf8(&self.result[..self.result_len]).unwrap_or("INVALID")
        };
        canvas.draw_text(output.x + 10, output.y + 31, result, 0x66D9EF, 1);

        for index in 0..self.buttons.len() {
            self.buttons[index].render(canvas, Self::button_rect(area, index), pointer);
        }
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Key(key) => self.press_key(key),
            Event::Pointer(pointer) => {
                let mut changed = false;
                for index in 0..self.buttons.len() {
                    if self.buttons[index].handle_pointer(Self::button_rect(area, index), pointer) {
                        changed |= self.press_key(KEY_VALUES[index]);
                    }
                }
                changed
            }
        }
    }
}

fn parse_integer(bytes: &[u8]) -> Result<i32, ()> {
    core::str::from_utf8(bytes)
        .map_err(|_| ())?
        .parse()
        .map_err(|_| ())
}
