// Graphical desktop shell for the NebulaOS x86 operating system.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#include "shell.hpp"
#include "../drivers/graphics.hpp"
#include "../drivers/keyboard.hpp"
#include "../drivers/mouse.hpp"
#include "../kernel/heap.hpp"
#include "../kernel/paging.hpp"
#include "../kernel/pmm.hpp"
#include "../kernel/timer.hpp"

namespace {
const unsigned int text_capacity = 40;

const unsigned int taskbar_height = 40;
const unsigned int start_button_x = 8;
const unsigned int start_button_y = 6;
const unsigned int start_button_width = 124;
const unsigned int start_button_height = 28;

const unsigned int menu_x = 8;
const unsigned int menu_y = taskbar_height + 4;
const unsigned int menu_width = 216;
const unsigned int menu_header_height = 26;
const unsigned int menu_item_height = 30;
const unsigned int no_item = 0xFFFFFFFF;
const unsigned int menu_item_count = 3;

const unsigned int card_x_margin = 24;
const unsigned int card_top = 64;
const unsigned int card_width = 296;
const unsigned int card_height = 96;

const unsigned int background = 0x0015202E;
const unsigned int taskbar_color = 0x00223A54;
const unsigned int accent = 0x0038BDF8;
const unsigned int panel_color = 0x00F1F5F9;
const unsigned int panel_title = 0x00334E68;
const unsigned int panel_border = 0x00B6C7D6;
const unsigned int dark_text = 0x00182736;
const unsigned int white = 0x00FFFFFF;
const unsigned int muted_text = 0x00C7D9E8;
const unsigned int dim_text = 0x00597A92;
const unsigned int warning = 0x00FFD080;

const char* const menu_labels[menu_item_count] = {
    "SYSTEM", "MEMORY", "ABOUT NEBULAOS"
};

bool start_menu_open = false;
unsigned int hovered_item = no_item;
bool start_button_hovered = false;
bool start_button_active = false;
int active_panel = -1;
bool mouse_present = false;
drivers::mouse::State mouse_state = {0, 0, false, false};

bool contains(unsigned int x, unsigned int y, unsigned int left, unsigned int top,
              unsigned int region_width, unsigned int region_height) {
    return x >= left && x < left + region_width && y >= top &&
           y < top + region_height;
}

unsigned int append(char* buffer, unsigned int offset, const char* text) {
    while (*text != '\0' && offset < text_capacity - 1) {
        buffer[offset] = *text;
        ++offset;
        ++text;
    }
    buffer[offset] = '\0';
    return offset;
}

unsigned int append_decimal(char* buffer, unsigned int offset, unsigned int value) {
    char digits[11];
    unsigned int count = 0;
    do {
        digits[count] = static_cast<char>('0' + (value % 10));
        ++count;
        value /= 10;
    } while (value != 0);
    while (count > 0 && offset < text_capacity - 1) {
        buffer[offset] = digits[--count];
        ++offset;
    }
    buffer[offset] = '\0';
    return offset;
}

void draw_card(const char* heading, const char* first, const char* second,
               const char* third) {
    const unsigned int x = card_x_margin;
    const unsigned int y = card_top;
    drivers::graphics::fill_rect(x, y, card_width, card_height, 0x00203A5A);
    drivers::graphics::fill_rect(x, y, card_width, 26, 0x002B4C70);
    drivers::graphics::draw_text(x + 16, y + 9, heading, white, 1);
    drivers::graphics::draw_text(x + 16, y + 38, first, muted_text, 1);
    drivers::graphics::draw_text(x + 16, y + 56, second, muted_text, 1);
    drivers::graphics::draw_text(x + 16, y + 74, third, dim_text, 1);
}

void draw_start_menu() {
    const unsigned int height = menu_header_height + menu_item_count * menu_item_height;
    drivers::graphics::fill_rect(menu_x + 3, menu_y + 3, menu_width, height, 0x00000000);
    drivers::graphics::fill_rect(menu_x, menu_y, menu_width, height, panel_color);
    drivers::graphics::fill_rect(menu_x, menu_y, menu_width, menu_header_height, panel_title);
    drivers::graphics::draw_text(menu_x + 12, menu_y + 9, "NEBULA MENU", white, 1);

    for (unsigned int index = 0; index < menu_item_count; ++index) {
        const unsigned int top =
            menu_y + menu_header_height + index * menu_item_height;
        if (index == hovered_item) {
            drivers::graphics::fill_rect(menu_x, top, menu_width, menu_item_height, accent);
        }
        drivers::graphics::draw_text(menu_x + 12, top + 11, menu_labels[index],
                                     dark_text, 1);
    }
}

void draw_panel_title(unsigned int left, unsigned int top, unsigned int width,
                      const char* title) {
    drivers::graphics::fill_rect(left, top, width, 36, panel_title);
    drivers::graphics::draw_text(left + 18, top + 13, title, white, 2);
}

void draw_system_panel(unsigned int screen_width, unsigned int screen_height) {
    char buffer[text_capacity];
    const unsigned int width = screen_width < 420 ? screen_width - 48 : 372;
    const unsigned int left = (screen_width - width) / 2;
    const unsigned int top = (screen_height - 250) / 2 + 30;

    unsigned int offset = 0;
    buffer[0] = '\0';
    offset = append(buffer, offset, "DISPLAY ");
    offset = append_decimal(buffer, offset, drivers::graphics::width());
    offset = append(buffer, offset, " X ");
    offset = append_decimal(buffer, offset, drivers::graphics::height());

    unsigned int row = top + 58;
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 20;
    drivers::graphics::draw_text(left + 20, row,
                                 drivers::graphics::is_double_buffered()
                                     ? "DOUBLE BUFFERING ACTIVE"
                                     : "DOUBLE BUFFERING UNAVAILABLE",
                                 dark_text, 1);
    row += 20;
    drivers::graphics::draw_text(left + 20, row,
                                 kernel::memory::paging::is_enabled()
                                     ? "PAGING ENABLED"
                                     : "PAGING DISABLED",
                                 dark_text, 1);
    row += 20;
    buffer[0] = '\0';
    offset = append(buffer, offset, "UPTIME ");
    offset = append_decimal(buffer, offset, kernel::timer::ticks() / 100);
    offset = append(buffer, offset, " S");
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 24;
    drivers::graphics::draw_text(left + 20, row, "CLICK OUTSIDE TO CLOSE", muted_text, 1);
}

void draw_memory_panel(unsigned int screen_width, unsigned int screen_height) {
    char buffer[text_capacity];
    const unsigned int width = screen_width < 420 ? screen_width - 48 : 372;
    const unsigned int left = (screen_width - width) / 2;
    const unsigned int top = (screen_height - 250) / 2 + 30;

    unsigned int row = top + 58;
    buffer[0] = '\0';
    unsigned int offset = append(buffer, 0, "PAGE FRAMES FREE ");
    offset = append_decimal(buffer, offset, kernel::memory::pmm::free_frames());
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 20;
    buffer[0] = '\0';
    offset = append(buffer, 0, "MANAGED ");
    offset = append_decimal(buffer, offset,
                            kernel::memory::pmm::managed_bytes() / (1024 * 1024));
    offset = append(buffer, offset, " MB");
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 20;
    buffer[0] = '\0';
    offset = append(buffer, 0, "HEAP TOTAL ");
    offset = append_decimal(buffer, offset, kernel::memory::heap::total_bytes() / 1024);
    offset = append(buffer, offset, " KB");
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 20;
    buffer[0] = '\0';
    offset = append(buffer, 0, "HEAP USED ");
    offset = append_decimal(buffer, offset, kernel::memory::heap::used_bytes() / 1024);
    offset = append(buffer, offset, " KB");
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 20;
    buffer[0] = '\0';
    offset = append(buffer, 0, "HEAP FREE ");
    offset = append_decimal(buffer, offset, kernel::memory::heap::free_bytes() / 1024);
    offset = append(buffer, offset, " KB");
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 20;
    buffer[0] = '\0';
    offset = append(buffer, 0, "IDENTITY MAP ");
    offset = append_decimal(buffer, offset, kernel::memory::paging::identity_megabytes());
    offset = append(buffer, offset, " MB");
    drivers::graphics::draw_text(left + 20, row, buffer, dark_text, 1);
    row += 24;
    drivers::graphics::draw_text(left + 20, row, "CLICK OUTSIDE TO CLOSE", muted_text, 1);
}

void draw_about_panel(unsigned int screen_width, unsigned int screen_height) {
    const char* const lines[] = {
        "NEBULAOS COPYRIGHT (C) 2026",
        "BY NEBULAJAPANESE-1221",
        "NEBULAJAPANESE@GMAIL.COM",
        "",
        "FREE SOFTWARE UNDER GPLV3",
        "OR LATER. SEE LICENCE IN THE",
        "SOURCE TREE FOR FULL TERMS.",
        "ABSOLUTELY NO WARRANTY."
    };
    const unsigned int line_count =
        static_cast<unsigned int>(sizeof(lines) / sizeof(lines[0]));
    const unsigned int width = screen_width < 420 ? screen_width - 48 : 372;
    const unsigned int left = (screen_width - width) / 2;
    const unsigned int top = (screen_height - 250) / 2 + 30;

    unsigned int row = top + 58;
    for (unsigned int index = 0; index < line_count; ++index) {
        if (lines[index][0] != '\0') {
            drivers::graphics::draw_text(left + 20, row, lines[index],
                                         index < 3 ? dark_text : muted_text, 1);
        }
        row += 18;
    }
}

void render() {
    const unsigned int screen_width = drivers::graphics::width();
    const unsigned int screen_height = drivers::graphics::height();

    drivers::graphics::clear(background);
    drivers::graphics::fill_rect(0, 0, screen_width, taskbar_height, taskbar_color);

    const unsigned int button_color =
        (start_button_active || start_menu_open)
            ? accent
            : (start_button_hovered ? 0x004A6A85 : 0x00334E68);
    drivers::graphics::fill_rect(start_button_x, start_button_y, start_button_width,
                                 start_button_height, button_color);
    drivers::graphics::draw_text(start_button_x + 16, start_button_y + 10, "START",
                                 start_button_active || start_menu_open ? dark_text : white, 1);
    drivers::graphics::draw_text(start_button_x + start_button_width + 16,
                                 start_button_y + 10, "NEBULA OS", white, 1);
    if (screen_width > 200) {
        drivers::graphics::draw_text(screen_width - 116, start_button_y + 10,
                                     "DESKTOP", dim_text, 1);
    }

    draw_card("SYSTEM READY", "GRAPHICAL DESKTOP", "MOUSE AND KEYBOARD POLLED",
              "SERIAL CONSOLE ONLINE");
    if (!mouse_present) {
        drivers::graphics::draw_text(card_x_margin, card_top + card_height + 24,
                                     "PS/2 MOUSE NOT AVAILABLE", warning, 1);
    }

    if (start_menu_open) {
        draw_start_menu();
    }

    if (active_panel >= 0) {
        const unsigned int width = screen_width < 420 ? screen_width - 48 : 372;
        const unsigned int left = (screen_width - width) / 2;
        const unsigned int top = (screen_height - 250) / 2 + 30;
        drivers::graphics::fill_rect(left + 3, top + 3, width, 250, 0x00000000);
        drivers::graphics::fill_rect(left, top, width, 250, panel_color);
        drivers::graphics::fill_rect(left, top, width, 2, panel_border);
        if (active_panel == 0) {
            draw_panel_title(left, top, width, "SYSTEM");
            draw_system_panel(screen_width, screen_height);
        } else if (active_panel == 1) {
            draw_panel_title(left, top, width, "MEMORY");
            draw_memory_panel(screen_width, screen_height);
        } else {
            draw_panel_title(left, top, width, "ABOUT");
            draw_about_panel(screen_width, screen_height);
        }
    }

    if (mouse_present) {
        drivers::graphics::fill_rect(mouse_state.x, mouse_state.y, 2, 14, 0x00000000);
        drivers::graphics::fill_rect(mouse_state.x, mouse_state.y, 10, 2, 0x00000000);
        drivers::graphics::fill_rect(mouse_state.x + 2, mouse_state.y + 2, 2, 8, white);
        drivers::graphics::fill_rect(mouse_state.x + 2, mouse_state.y + 2, 6, 2, white);
        drivers::graphics::fill_rect(mouse_state.x + 4, mouse_state.y + 4, 2, 4, white);
        drivers::graphics::fill_rect(mouse_state.x + 6, mouse_state.y + 6, 2, 2, white);
    }

    drivers::graphics::present();
}

void update_hover() {
    start_button_hovered =
        contains(mouse_state.x, mouse_state.y, start_button_x, start_button_y,
                 start_button_width, start_button_height);

    hovered_item = no_item;
    if (!start_menu_open) {
        return;
    }
    for (unsigned int index = 0; index < menu_item_count; ++index) {
        const unsigned int top =
            menu_y + menu_header_height + index * menu_item_height;
        if (contains(mouse_state.x, mouse_state.y, menu_x, top, menu_width,
                     menu_item_height)) {
            hovered_item = index;
            return;
        }
    }
}

void close_panels() {
    active_panel = -1;
}

void handle_click() {
    if (!mouse_state.left_clicked) {
        return;
    }

    if (active_panel >= 0) {
        close_panels();
        return;
    }

    if (start_menu_open && hovered_item != no_item) {
        active_panel = static_cast<int>(hovered_item);
        start_menu_open = false;
        hovered_item = no_item;
        return;
    }

    if (start_button_hovered) {
        start_menu_open = !start_menu_open;
        hovered_item = no_item;
        return;
    }

    start_menu_open = false;
    hovered_item = no_item;
}
}

namespace shell {

[[noreturn]] void run(bool mouse_available) {
    mouse_present = mouse_available;
    render();

    for (;;) {
        bool redraw = false;

        if (drivers::mouse::poll(mouse_state)) {
            update_hover();
            handle_click();
            redraw = true;
        }

        char character;
        if (drivers::keyboard::try_read_character(character)) {
            if (character == '\n') {
                start_menu_open = !start_menu_open;
                hovered_item = no_item;
                redraw = true;
            } else if (character == '\b') {
                if (start_menu_open) {
                    start_menu_open = false;
                } else {
                    close_panels();
                }
                hovered_item = no_item;
                redraw = true;
            }
        }

        if (redraw) {
            render();
        }
        asm volatile("pause");
    }
}

}