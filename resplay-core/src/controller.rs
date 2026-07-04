use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Hash)]
    pub struct Button: u8 {
        const A = 1 << 0;
        const B = 1 << 1;
        const SELECT = 1 << 2;
        const START = 1 << 3;
        const UP = 1 << 4;
        const DOWN = 1 << 5;
        const LEFT = 1 << 6;
        const RIGHT = 1 << 7;
    }
}

impl Button {
    pub fn name(self) -> &'static str {
        match self {
            Button::A => "A",
            Button::B => "B",
            Button::SELECT => "Select",
            Button::START => "Start",
            Button::DOWN => "Down",
            Button::UP => "Up",
            Button::LEFT => "Left",
            Button::RIGHT => "Right",
            _ => unreachable!(),
        }
    }
}

#[derive(Default)]
pub struct Controller {
    strobe_active: bool,
    shift_register: u8,
    /// Bit flag of all button held down states
    state: Button,
}

impl Controller {
    pub fn write(&mut self, value: u8) {
        self.strobe_active = value & 0b1 != 0;
        if !self.strobe_active {
            self.shift_register = self.state.bits();
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.strobe_active {
            // Always return A when strobe active
            self.state.contains(Button::A) as u8
        } else {
            let bit = self.shift_register & 0b1;
            // Shift right and add 1 into high bit so it will read one when all bits have been read
            self.shift_register >>= 1;
            self.shift_register |= 0b1000_0000;
            bit
        }
    }

    /// Set a button to be pressed or not while checking if pressing the specified button will
    /// result in an illegal button press on a standard NES controller if allow_illegal_press is
    /// false. That being left and right or up and down at the same time.
    pub fn set_button(&mut self, button: Button, value: bool, allow_illegal_press: bool) {
        let pressing_illegal = self.check_pressed_combo(button, Button::LEFT, Button::RIGHT)
            || self.check_pressed_combo(button, Button::UP, Button::DOWN);
        if allow_illegal_press || !pressing_illegal {
            self.state.set(button, value);
        }
    }

    fn check_pressed_combo(&self, button: Button, a: Button, b: Button) -> bool {
        (button == a && self.state.contains(b)) || (button == b && self.state.contains(a))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_correct() {
        let mut con = Controller::default();
        con.write(1);
        assert_eq!(con.read(), 0);
        con.state.set(Button::A, true);
        assert_eq!(con.read(), 1);

        con.state.set(Button::SELECT, true);
        con.write(0);
        con.state.set(Button::B, true);
        let mut out = 0;
        for i in 0..10 {
            out |= (con.read() as u16) << i;
        }
        assert_eq!(out, 0b11_0000_0101);
    }
}
