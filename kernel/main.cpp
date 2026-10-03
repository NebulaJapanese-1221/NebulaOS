#include "../drivers/graphics.hpp"
#include "../drivers/keyboard.hpp"
#include "../shell/shell.hpp"

extern "C" void kmain(unsigned int boot_magic, unsigned int multiboot_info_address) {
    if (boot_magic != 0x2BADB002 ||
        !drivers::graphics::initialize(multiboot_info_address)) {
        for (;;) {
            asm volatile("cli; hlt");
        }
    }
    drivers::keyboard::initialize();
    shell::run();
}
