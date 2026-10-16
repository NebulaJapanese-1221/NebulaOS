#pragma once
namespace kernel::clipboard { void init(); void set(const char*); const char* get(); }
namespace kernel::dragdrop { void init(); }
namespace kernel::fonts { void init(); }
namespace kernel::acpi { bool init(); void shutdown(); void reboot(); void sleep(); }
namespace kernel::seriallog { void init(); void write(const char*); }
namespace kernel::panic { void show(const char*); }
namespace kernel::gdb { void init(); }
namespace kernel::installer { bool install(const char*); }
namespace kernel::notify { void init(); void push(const char*); }
namespace kernel::kbdlayout { void set(const char*); }
namespace kernel::audio { bool init(); void beep(unsigned int freq, unsigned int ms); }
namespace kernel::net { bool init(); }
namespace kernel::elf2 { void init(); }
