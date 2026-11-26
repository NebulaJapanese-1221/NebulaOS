// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

use super::gui::{Application, Button, Canvas, Event, PointerEvent, Rect, TextField};
use crate::libc;

const VOLUME_LABELS: [&str; 4] = ["MUTE", "LOW", "MED", "HIGH"];
const VOLUME_LEVELS: [u8; 4] = [0, 1, 2, 3];
const NOTE_NAMES: [&str; 8] = ["C", "D", "E", "F", "G", "A", "B", "C5"];

pub struct Mixer {
    volume: u8,
    ac97_available: bool,
    buttons: [Button; 4],
    volume_slider: Rect,
    slider_dragging: bool,
    test_tone_button: Button,
    startup_chime_button: Button,
    beep_button: Button,
    stop_button: Button,
    note_buttons: [Button; 8],
    last_error: Option<&'static str>,
}

impl Mixer {
    pub fn new() -> Self {
        let volume = unsafe { libc::audio_volume_get().unwrap_or(3) };
        Self {
            volume,
            ac97_available: unsafe { libc::audio_ac97_available().unwrap_or(false) },
            buttons: [
                Button::new("MUTE"),
                Button::new("LOW"),
                Button::new("MED"),
                Button::new("HIGH"),
            ],
            volume_slider: Rect { x: 20, y: 180, width: 280, height: 20 },
            slider_dragging: false,
            test_tone_button: Button::new("TEST TONE"),
            startup_chime_button: Button::new("STARTUP CHIME"),
            beep_button: Button::new("BEEP"),
            stop_button: Button::new("STOP"),
            note_buttons: [
                Button::new("C"),
                Button::new("D"),
                Button::new("E"),
                Button::new("F"),
                Button::new("G"),
                Button::new("A"),
                Button::new("B"),
                Button::new("C5"),
            ],
            last_error: None,
        }
    }

    fn set_volume(&mut self, level: u8) {
        self.volume = level.min(3);
        if let Err(_) = unsafe { libc::audio_volume_set(self.volume) } {
            self.last_error = Some("Failed to set volume");
        } else {
            self.last_error = None;
        }
        for (i, btn) in self.buttons.iter_mut().enumerate() {
            btn.armed = (i as u8) == self.volume;
        }
    }

    fn play_test_tone(&mut self) {
        if let Err(_) = unsafe { libc::audio_play_tone(440, 500) } {
            self.last_error = Some("Failed to play tone");
        } else {
            self.last_error = None;
        }
    }

    fn play_startup_chime(&mut self) {
        if let Err(_) = unsafe { libc::audio_startup_chime() } {
            self.last_error = Some("Failed to play chime");
        } else {
            self.last_error = None;
        }
    }

    fn play_beep(&mut self) {
        if let Err(_) = unsafe { libc::audio_beep() } {
            self.last_error = Some("Failed to beep");
        } else {
            self.last_error = None;
        }
    }

    fn play_note(&mut self, note_index: usize) {
        let frequencies = [262, 294, 330, 349, 392, 440, 494, 523];
        if note_index < frequencies.len() {
            if let Err(_) = unsafe { libc::audio_play_tone(frequencies[note_index], 200) } {
                self.last_error = Some("Failed to play note");
            } else {
                self.last_error = None;
            }
        }
    }

    fn stop_audio(&mut self) {
        if let Err(_) = unsafe { libc::audio_stop() } {
            self.last_error = Some("Failed to stop");
        } else {
            self.last_error = None;
        }
    }

    fn handle_slider(&mut self, pointer: PointerEvent, area: Rect) -> bool {
        let slider_x = area.x + self.volume_slider.x;
        let slider_y = area.y + self.volume_slider.y;
        let slider_right = slider_x + self.volume_slider.width;

        if pointer.left_down {
            if pointer.x >= slider_x && pointer.x <= slider_right
                && pointer.y >= slider_y && pointer.y <= slider_y + self.volume_slider.height {
                self.slider_dragging = true;
            }
        } else {
            self.slider_dragging = false;
        }

        if self.slider_dragging {
            let rel_x = pointer.x.saturating_sub(slider_x).min(self.volume_slider.width);
            let level = (rel_x * 4 / self.volume_slider.width).min(3) as u8;
            self.set_volume(level);
            return true;
        }
        false
    }
}

impl Application for Mixer {
    fn title(&self) -> &'static str {
        "MIXER"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        let bg_color = 0xE2E8F0;
        let panel_color = 0xFFFFFF;
        let accent = 0x1596B5;
        let text_color = 0x111827;
        let muted = 0x516786;
        let border = 0x516786;

        canvas.fill_rect(area, bg_color);

        let title_y = area.y + 20;
        canvas.draw_text(area.x + 20, title_y, "AUDIO MIXER", text_color, 2);

        let status_y = area.y + 60;
        let ac97_status = if self.ac97_available { "AC'97: YES" } else { "AC'97: NO (PC Speaker only)" };
        canvas.draw_text(area.x + 20, status_y, ac97_status, if self.ac97_available { 0x059669 } else { 0xDC2626 }, 1);

        let vol_y = area.y + 90;
        canvas.draw_text(area.x + 20, vol_y, "VOLUME:", text_color, 1);

        for (i, btn) in self.buttons.iter().enumerate() {
            let btn_x = area.x + 20 + (i as u32 * 80);
            let btn_y = area.y + 120;
            let btn_rect = Rect { x: btn_x, y: btn_y, width: 70, height: 36 };
            let is_active = (i as u8) == self.volume;
            let btn_color = if is_active { accent } else { panel_color };
            canvas.fill_rect(btn_rect, btn_color);
            canvas.draw_rect(btn_rect, if is_active { accent } else { border });
            let label = VOLUME_LABELS[i];
            let label_x = btn_x + (70 - label.len() as u32 * 6) / 2;
            canvas.draw_text(label_x, btn_y + 12, label, if is_active { 0xFFFFFF } else { text_color }, 1);
        }

        let slider_bg = Rect {
            x: area.x + self.volume_slider.x,
            y: area.y + self.volume_slider.y,
            width: self.volume_slider.width,
            height: self.volume_slider.height,
        };
        canvas.fill_rect(slider_bg, 0xCBD5E1);
        canvas.draw_rect(slider_bg, border);

        let fill_width = ((self.volume as u32 + 1) * self.volume_slider.width) / 4;
        let slider_fill = Rect {
            x: area.x + self.volume_slider.x,
            y: area.y + self.volume_slider.y,
            width: fill_width,
            height: self.volume_slider.height,
        };
        canvas.fill_rect(slider_fill, accent);

        let handle_x = area.x + self.volume_slider.x + (self.volume as u32 * self.volume_slider.width) / 3;
        let handle = Rect { x: handle_x.saturating_sub(4), y: area.y + self.volume_slider.y - 4, width: 8, height: 28 };
        canvas.fill_rect(handle, 0xFFFFFF);
        canvas.draw_rect(handle, accent);

        let test_y = area.y + 230;
        let test_buttons = [
            (&self.test_tone_button, "TEST TONE (440Hz)", 20),
            (&self.startup_chime_button, "STARTUP CHIME", 20),
            (&self.beep_button, "BEEP", 20),
            (&self.stop_button, "STOP", 20),
        ];
        for (i, (btn, label, btn_y)) in test_buttons.iter().enumerate() {
            let x = area.x + 20 + (i as u32 * 85);
            let y = test_y;
            let rect = Rect { x, y, width: 80, height: 36 };
            let color = if btn.armed { accent } else { panel_color };
            canvas.fill_rect(rect, color);
            canvas.draw_rect(rect, border);
            canvas.draw_text(x + 5, y + 12, label, text_color, 1);
        }

        let piano_y = area.y + 290;
        canvas.draw_text(area.x + 20, piano_y - 20, "PIANO:", text_color, 1);
        for (i, btn) in self.note_buttons.iter().enumerate() {
            let x = area.x + 20 + (i as u32 * 40);
            let y = piano_y;
            let is_black = matches!(i, 1 | 3 | 5 | 6);
            let rect = Rect { x, y, width: 36, height: 50 };
            let color = if btn.armed { accent } else if is_black { 0x111827 } else { 0xFFFFFF };
            let text_col = if is_black { 0xFFFFFF } else { 0x111827 };
            canvas.fill_rect(rect, color);
            canvas.draw_rect(rect, border);
            canvas.draw_text(x + 10, y + 18, NOTE_NAMES[i], text_col, 1);
        }

        if let Some(err) = self.last_error {
            let err_y = area.y + area.height - 40;
            canvas.draw_text(area.x + 20, err_y, err, 0xDC2626, 1);
        }
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Key(key) => {
                match key {
                    b'm' | b'M' => {
                        self.set_volume(0);
                        true
                    }
                    b'l' | b'L' => {
                        self.set_volume(1);
                        true
                    }
                    b'=' | b'+' => {
                        if self.volume < 3 { self.set_volume(self.volume + 1); }
                        true
                    }
                    b'-' | b'_' => {
                        if self.volume > 0 { self.set_volume(self.volume - 1); }
                        true
                    }
                    b't' | b'T' => { self.play_test_tone(); true }
                    b's' | b'S' => { self.play_startup_chime(); true }
                    b'b' | b'B' => { self.play_beep(); true }
                    b'x' | b'X' => { self.stop_audio(); true }
                    b'1'..=b'8' => {
                        let note_idx = (key - b'1') as usize;
                        self.play_note(note_idx);
                        true
                    }
                    _ => false,
                }
            }
            Event::Pointer(pointer) => {
                let mut handled = false;

                for (i, btn) in self.buttons.iter_mut().enumerate() {
                    let btn_x = area.x + 20 + (i as u32 * 80);
                    let btn_y = area.y + 120;
                    let btn_rect = Rect { x: btn_x, y: btn_y, width: 70, height: 36 };
                    let was_armed = btn.armed;
                    btn.armed = btn_rect.contains(pointer.x, pointer.y);
                    if was_armed != btn.armed && pointer.left_down && btn.armed {
                        self.set_volume(i as u8);
                        handled = true;
                    }
                }

                if !handled {
                    handled = self.handle_slider(pointer, area);
                }

                let test_y = area.y + 230;
                let test_btns = [
                    &mut self.test_tone_button,
                    &mut self.startup_chime_button,
                    &mut self.beep_button,
                    &mut self.stop_button,
                ];
                for (i, btn) in test_btns.iter_mut().enumerate() {
                    let x = area.x + 20 + (i as u32 * 85);
                    let rect = Rect { x, y: test_y, width: 80, height: 36 };
                    let was_armed = btn.armed;
                    btn.armed = rect.contains(pointer.x, pointer.y);
                    if was_armed != btn.armed && !pointer.left_down && was_armed {
                        match i {
                            0 => self.play_test_tone(),
                            1 => self.play_startup_chime(),
                            2 => self.play_beep(),
                            3 => self.stop_audio(),
                        }
                        handled = true;
                    }
                }

                let piano_y = area.y + 290;
                for (i, btn) in self.note_buttons.iter_mut().enumerate() {
                    let x = area.x + 20 + (i as u32 * 40);
                    let rect = Rect { x, y: piano_y, width: 36, height: 50 };
                    let was_armed = btn.armed;
                    btn.armed = rect.contains(pointer.x, pointer.y);
                    if was_armed != btn.armed && !pointer.left_down && was_armed {
                        self.play_note(i);
                        handled = true;
                    }
                }

                handled
            }
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        let vol_y = area.y + 120;
        for i in 0..4 {
            let btn_x = area.x + 20 + (i as u32 * 80);
            let rect = Rect { x: btn_x, y: vol_y, width: 70, height: 36 };
            if rect.contains(x, y) { return Some(1); }
        }
        let slider_bg = Rect {
            x: area.x + self.volume_slider.x,
            y: area.y + self.volume_slider.y,
            width: self.volume_slider.width,
            height: self.volume_slider.height,
        };
        if slider_bg.contains(x, y) { return Some(1); }
        let test_y = area.y + 230;
        for i in 0..4 {
            let btn_x = area.x + 20 + (i as u32 * 85);
            let rect = Rect { x: btn_x, y: test_y, width: 80, height: 36 };
            if rect.contains(x, y) { return Some(1); }
        }
        let piano_y = area.y + 290;
        for i in 0..8 {
            let btn_x = area.x + 20 + (i as u32 * 40);
            let rect = Rect { x: btn_x, y: piano_y, width: 36, height: 50 };
            if rect.contains(x, y) { return Some(1); }
        }
        None
    }
}