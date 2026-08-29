use crate::common::io;
use crate::common::vga;

static mut PCI_DEVICES: [PciDevice; 256] = [PciDevice::zero(); 256];
static mut PCI_DEVICE_COUNT: u32 = 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub header_type: u8,
    pub bar: [u32; 6],
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
}

impl PciDevice {
    const fn zero() -> Self {
        PciDevice { bus: 0, device: 0, function: 0, vendor_id: 0, device_id: 0, class_code: 0, subclass: 0, prog_if: 0, header_type: 0, bar: [0; 6], interrupt_line: 0, interrupt_pin: 0 }
    }
}

static mut PCI_DRIVERS: *mut PciDriver = core::ptr::null_mut();

#[repr(C)]
pub struct PciDriver {
    pub vendor_id: u16,
    pub device_id: u16,
    pub name: *const u8,
    pub init: unsafe extern "C" fn(*mut PciDevice) -> i32,
    pub next: *mut PciDriver,
}

#[no_mangle]
pub unsafe extern "C" fn pci_init() {
    vga::puts(b"Initializing PCI subsystem...\n\0" as *const u8 as *const u8);
    pci_enumerate_bus();
}

unsafe fn pci_device_exists(bus: u8, device: u8, function: u8) -> bool {
    pci_read_config_word(bus, device, function, 0x00) != 0xFFFF
}

unsafe fn pci_read_config_byte(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let address = 0x80000000 | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | ((offset as u32) & 0xFC);
    io::outl(0xCF8, address);
    io::inb(0xCFC + ((offset & 3) as u16))
}

unsafe fn pci_read_config_word(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let address = 0x80000000 | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | ((offset as u32) & 0xFC);
    io::outl(0xCF8, address);
    io::inw(0xCFC + ((offset & 2) as u16))
}

unsafe fn pci_read_config_dword(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = 0x80000000 | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | ((offset as u32) & 0xFC);
    io::outl(0xCF8, address);
    io::inl(0xCFC)
}

unsafe fn pci_enumerate_bus() {
    PCI_DEVICE_COUNT = 0;
    vga::puts(b"PCI: Enumerating bus...\n\0" as *const u8 as *const u8);
    for bus in 0..256 {
        for device in 0..32 {
            if !pci_device_exists(bus, device, 0) {
                continue;
            }
            for function in 0..8 {
                if !pci_device_exists(bus, device, function) {
                    continue;
                }
                if PCI_DEVICE_COUNT >= 256 {
                    break;
                }
                let pdev = &mut *PCI_DEVICES.as_mut_ptr().add(PCI_DEVICE_COUNT as usize);
                pdev.bus = bus;
                pdev.device = device;
                pdev.function = function;
                pdev.vendor_id = pci_read_config_word(bus, device, function, 0x00);
                pdev.device_id = pci_read_config_word(bus, device, function, 0x02);
                pdev.class_code = pci_read_config_byte(bus, device, function, 0x0B);
                pdev.subclass = pci_read_config_byte(bus, device, function, 0x0A);
                pdev.header_type = pci_read_config_byte(bus, device, function, 0x0E);
                for i in 0..6 {
                    pdev.bar[i] = pci_read_config_dword(bus, device, function, 0x10 + i * 4);
                }
                PCI_DEVICE_COUNT += 1;
            }
        }
    }
    vga::puts(b"PCI: Found \0" as *const u8 as *const u8);
    let mut count = PCI_DEVICE_COUNT;
    let mut buf = [0u8; 16];
    let mut i = 0;
    while count > 0 && i < 15 {
        buf[i] = (count % 10) as u8 + b'0';
        count /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        vga::putchar(buf[i] as char);
    }
    vga::puts(b" devices\n\0" as *const u8 as *const u8);
}
