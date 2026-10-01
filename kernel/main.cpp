#include "../shell/shell.hpp"
#include "../drivers/keyboard.hpp"
#include "../drivers/vga.hpp"

extern "C" void kmain() {
    drivers::vga::initialize();
    drivers::keyboard::initialize();
    shell::run();
}
