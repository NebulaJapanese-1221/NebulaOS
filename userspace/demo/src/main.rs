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

#![no_std]
#![no_main]

#[path = "../../../kernel/memory/mem.rs"]
mod memory_routines;
#[path = "../../storage.rs"]
mod storage;
#[path = "../../mod.rs"]
mod userspace;

pub use userspace::syscall;

use userspace::apps::{
    calculator_gui::Calculator,
    editor_gui::TextEditor,
    files_gui::FileManager,
    gui::{Application, Canvas, Event, PointerEvent, Rect},
    monitor_gui::SystemMonitor,
    paint_gui::Paint,
    settings_gui::Settings,
    terminal_gui::Terminal,
    userdemo_gui::UserModeDemo,
};

enum UserApp {
    Calculator(Calculator),
    Editor(TextEditor),
    Files(FileManager),
    Settings(Settings),
    Paint(Paint),
    Monitor(SystemMonitor),
    UserMode(UserModeDemo),
    Terminal(Terminal),
}

impl UserApp {
    fn new(app_id: u32, context: syscall::AppContext) -> Option<Self> {
        Some(match app_id {
            0 => Self::Calculator(Calculator::new()),
            1 => Self::Editor(TextEditor::new()),
            2 => Self::Files(FileManager::new()),
            3 => Self::Settings(Settings::new_with_state(
                context.dark_theme != 0,
                context.mouse_sensitivity,
            )),
            4 => Self::Paint(Paint::new()),
            5 => Self::Monitor(SystemMonitor::new()),
            6 => Self::UserMode(UserModeDemo::new()),
            7 => Self::Terminal(Terminal::new()),
            _ => return None,
        })
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        match self {
            Self::Calculator(app) => app.render(canvas, area, pointer),
            Self::Editor(app) => app.render(canvas, area, pointer),
            Self::Files(app) => app.render(canvas, area, pointer),
            Self::Settings(app) => app.render(canvas, area, pointer),
            Self::Paint(app) => app.render(canvas, area, pointer),
            Self::Monitor(app) => app.render(canvas, area, pointer),
            Self::UserMode(app) => app.render(canvas, area, pointer),
            Self::Terminal(app) => app.render(canvas, area, pointer),
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Calculator(app) => app.title(),
            Self::Editor(app) => app.title(),
            Self::Files(app) => app.title(),
            Self::Settings(app) => app.title(),
            Self::Paint(app) => app.title(),
            Self::Monitor(app) => app.title(),
            Self::UserMode(app) => app.title(),
            Self::Terminal(app) => app.title(),
        }
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match self {
            Self::Calculator(app) => app.handle_event(event, area),
            Self::Editor(app) => app.handle_event(event, area),
            Self::Files(app) => app.handle_event(event, area),
            Self::Settings(app) => app.handle_event(event, area),
            Self::Paint(app) => app.handle_event(event, area),
            Self::Monitor(app) => app.handle_event(event, area),
            Self::UserMode(app) => app.handle_event(event, area),
            Self::Terminal(app) => app.handle_event(event, area),
        }
    }

    fn pointer_target(&self, x: u32, y: u32, area: Rect) -> Option<u32> {
        match self {
            Self::Calculator(app) => app.pointer_target(x, y, area),
            Self::Editor(app) => app.pointer_target(x, y, area),
            Self::Files(app) => app.pointer_target(x, y, area),
            Self::Settings(app) => app.pointer_target(x, y, area),
            Self::Paint(app) => app.pointer_target(x, y, area),
            Self::Monitor(app) => app.pointer_target(x, y, area),
            Self::UserMode(app) => app.pointer_target(x, y, area),
            Self::Terminal(app) => app.pointer_target(x, y, area),
        }
    }
}

struct AppCanvas {
    failed: bool,
    area: Rect,
}

impl Canvas for AppCanvas {
    fn fill_rect(&mut self, rect: Rect, color: u32) {
        self.draw(rect, color, 0);
    }

    fn draw_rect(&mut self, rect: Rect, color: u32) {
        self.draw(rect, color, 1);
    }

    fn draw_text(&mut self, x: u32, y: u32, text: &str, color: u32, scale: u32) {
        if self.failed {
            return;
        }
        let scale = scale.max(1);
        let relative_x = x.saturating_sub(self.area.x);
        let relative_y = y.saturating_sub(self.area.y);
        let cell_width = 6u32.saturating_mul(scale);
        if relative_x >= self.area.width || relative_y >= self.area.height {
            return;
        }
        let visible_chars = (self.area.width - relative_x) / cell_width;
        let mut text_len = text.len().min(visible_chars as usize);
        while !text.is_char_boundary(text_len) {
            text_len -= 1;
        }
        if text_len == 0 || 8u32.saturating_mul(scale) > self.area.height - relative_y {
            return;
        }
        let text = &text[..text_len];
        let text_width = (text_len as u32).saturating_mul(cell_width);
        let mut command = syscall::DrawCommand {
            kind: 2,
            x: relative_x,
            y: relative_y,
            width: text_width,
            height: 8u32.saturating_mul(scale),
            color,
            scale,
            text_ptr: text.as_ptr() as u32,
            text_len: text.len() as u32,
            ..syscall::DrawCommand::default()
        };
        self.failed |= syscall::draw(&mut command).is_err();
    }
}

impl AppCanvas {
    fn draw(&mut self, rect: Rect, color: u32, kind: u32) {
        if self.failed {
            return;
        }
        let right = rect
            .x
            .saturating_add(rect.width)
            .min(self.area.x.saturating_add(self.area.width));
        let bottom = rect
            .y
            .saturating_add(rect.height)
            .min(self.area.y.saturating_add(self.area.height));
        let x = rect.x.max(self.area.x);
        let y = rect.y.max(self.area.y);
        if right <= x || bottom <= y {
            return;
        }
        let mut command = syscall::DrawCommand {
            kind,
            x: x - self.area.x,
            y: y - self.area.y,
            width: right - x,
            height: bottom - y,
            color,
            ..syscall::DrawCommand::default()
        };
        self.failed |= syscall::draw(&mut command).is_err();
    }
}

#[no_mangle]
pub extern "C" fn _start(app_id: u32, x: u32, y: u32, width: u32, height: u32) -> ! {
    let mut context = syscall::AppContext::default();
    if syscall::app_context(&mut context).is_err() {
        terminate(1);
    }
    let Some(mut app) = UserApp::new(app_id, context) else {
        terminate(1);
    };
    if syscall::write(app.title().as_bytes()).is_err() {
        terminate(1);
    }
    let mut area = Rect {
        x,
        y,
        width,
        height,
    };
    let mut last_pointer = empty_pointer();
    let mut canvas = AppCanvas {
        failed: false,
        area,
    };
    app.render(&mut canvas, area, empty_pointer());
    if canvas.failed {
        terminate(1);
    }

    loop {
        let mut event = syscall::AppEvent::default();
        match syscall::poll_event(&mut event) {
            Ok(false) => continue,
            Err(_) => terminate(1),
            Ok(true) => {}
        }

        match event.kind {
            1 => {
                if app.handle_event(Event::Key(event.key as u8), area) {
                    render(&app, area, last_pointer);
                }
            }
            2 => {
                let pointer = event_pointer(event);
                let old_target = app.pointer_target(last_pointer.x, last_pointer.y, area);
                let button_changed = last_pointer.left_down != pointer.left_down;
                app.handle_event(Event::Pointer(pointer), area);
                let target_changed = old_target != app.pointer_target(pointer.x, pointer.y, area);
                last_pointer = pointer;
                if button_changed || target_changed {
                    render(&app, area, pointer);
                }
            }
            3 => {
                let mut context = syscall::AppContext::default();
                if syscall::app_context(&mut context).is_err() {
                    terminate(1);
                }
                area = Rect {
                    x: context.x,
                    y: context.y,
                    width: context.width,
                    height: context.height,
                };
                last_pointer = event_pointer(event);
                render(&app, area, last_pointer);
            }
            _ => {}
        }
    }
}

fn render(app: &UserApp, area: Rect, pointer: PointerEvent) {
    let mut canvas = AppCanvas {
        failed: false,
        area,
    };
    app.render(&mut canvas, area, pointer);
    if canvas.failed {
        terminate(1);
    }
}

fn empty_pointer() -> PointerEvent {
    PointerEvent {
        x: 0,
        y: 0,
        left_down: false,
    }
}

fn event_pointer(event: syscall::AppEvent) -> PointerEvent {
    PointerEvent {
        x: event.x,
        y: event.y,
        left_down: event.left_down != 0,
    }
}

fn terminate(code: u32) -> ! {
    let _ = syscall::exit(code);
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    terminate(u32::MAX)
}
