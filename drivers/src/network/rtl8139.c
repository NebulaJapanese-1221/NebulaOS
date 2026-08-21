// NebulaOS - RTL8139 Network Driver
// ===================================
//
// RTL8139 PCI network driver (stub implementation)

#include "../include/network.h"
#include "../include/pci.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/vga.h"
#include "../../lib/include/string.h"

// -----------------------------------------------------------------------------
// I/O functions
// -----------------------------------------------------------------------------

static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" : : "a"(value), "dN"(port));
}

static inline uint16_t inw(uint16_t port) {
    uint16_t value;
    __asm__ __volatile__("inw %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outw(uint16_t port, uint16_t value) {
    __asm__ __volatile__("outw %0, %1" : : "a"(value), "dN"(port));
}

static inline uint32_t inl(uint16_t port) {
    uint32_t value;
    __asm__ __volatile__("inl %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outl(uint16_t port, uint32_t value) {
    __asm__ __volatile__("outl %0, %1" : : "a"(value), "dN"(port));
}

// -----------------------------------------------------------------------------
// RTL8139 device state
// -----------------------------------------------------------------------------

static rtl8139_device_t rtl8139_dev;

// -----------------------------------------------------------------------------
// Read MAC address
// -----------------------------------------------------------------------------

void rtl8139_read_mac(rtl8139_device_t* dev) {
    uint32_t* mac = (uint32_t*)dev->mac;
    mac[0] = inl(dev->io_base + RTL8139_MAC0);
    mac[1] = inw(dev->io_base + RTL8139_MAC0 + 4);
}

// -----------------------------------------------------------------------------
// Enable interrupts
// -----------------------------------------------------------------------------

void rtl8139_enable_interrupts(rtl8139_device_t* dev) {
    outw(dev->io_base + RTL8139_INT_MASK,
         RTL8139_INT_TX_OK | RTL8139_INT_RX_OK | RTL8139_INT_RX_OVERFLOW);
}

// -----------------------------------------------------------------------------
// Disable interrupts
// -----------------------------------------------------------------------------

void rtl8139_disable_interrupts(rtl8139_device_t* dev) {
    outw(dev->io_base + RTL8139_INT_MASK, 0);
}

// -----------------------------------------------------------------------------
// Start receiver
// -----------------------------------------------------------------------------

void rtl8139_start_receiver(rtl8139_device_t* dev) {
    outb(dev->io_base + RTL8139_CMD, RTL8139_CMD_RX_EN);
}

// -----------------------------------------------------------------------------
// Stop receiver
// -----------------------------------------------------------------------------

void rtl8139_stop_receiver(rtl8139_device_t* dev) {
    outb(dev->io_base + RTL8139_CMD, 0);
}

// -----------------------------------------------------------------------------
// Reset RTL8139
// -----------------------------------------------------------------------------

void rtl8139_reset(rtl8139_device_t* dev) {
    outb(dev->io_base + RTL8139_CMD, RTL8139_CMD_RESET);

    // Wait for reset to complete
    for (int i = 0; i < 100; i++) {
        if (!(inb(dev->io_base + RTL8139_CMD) & RTL8139_CMD_RESET)) {
            break;
        }
    }
}

// -----------------------------------------------------------------------------
// Initialize RTL8139
// -----------------------------------------------------------------------------

int rtl8139_init(rtl8139_device_t* dev) {
    if (!dev) {
        return -1;
    }

    char buf[128];
    snprintf(buf, sizeof(buf), "RTL8139: Initializing at I/O 0x%04X\n", dev->io_base);
    vga_puts(buf);

    // Reset the chip
    rtl8139_reset(dev);

    // Read MAC address
    rtl8139_read_mac(dev);
    snprintf(buf, sizeof(buf), "RTL8139: MAC %02X:%02X:%02X:%02X:%02X:%02X\n",
        dev->mac[0], dev->mac[1], dev->mac[2],
        dev->mac[3], dev->mac[4], dev->mac[5]);
    vga_puts(buf);

    // Enable bus mastering
    uint16_t cmd = pci_read_config_word(dev->io_base >> 8, 0, 0, 0x04);
    cmd |= 0x02 | 0x04;  // Bus master enable + memory enable
    pci_write_config_word(dev->io_base >> 8, 0, 0, 0x04, cmd);

    // Disable interrupts during setup
    rtl8139_disable_interrupts(dev);

    // Set receive mode (accept broadcast and physical match)
    outb(dev->io_base + RTL8139_RX_MODE,
         RTL8139_RX_ACCEPT_BROADCAST | RTL8139_RX_ACCEPT_PHYS_MATCH);

    // Enable transmitter and receiver
    outb(dev->io_base + RTL8139_CMD,
         RTL8139_CMD_TX_EN | RTL8139_CMD_RX_EN | RTL8139_CMD_BUS_MASTER);

    // Enable interrupts
    rtl8139_enable_interrupts(dev);

    // Initialize transmit buffers
    dev->current_tx = 0;
    for (int i = 0; i < 4; i++) {
        dev->tx_offsets[i] = 0x10 + i * 4;
    }

    dev->initialized = true;
    vga_puts("RTL8139: Initialized\n");

    return 0;
}

// -----------------------------------------------------------------------------
// Send packet
// -----------------------------------------------------------------------------

int rtl8139_send(rtl8139_device_t* dev, const uint8_t* data, uint16_t length) {
    if (!dev || !dev->initialized || !data || length == 0) {
        return -1;
    }

    uint32_t offset = dev->tx_offsets[dev->current_tx];

    // Write packet to transmit buffer
    for (uint16_t i = 0; i < length && i < RTL8139_TX_BUF_SIZE; i++) {
        outb(dev->io_base + offset, data[i]);
    }

    // Set transmit status (length + TX_OK | OWNED)
    outl(dev->io_base + RTL8139_TX_STATUS0 + dev->current_tx * 4,
         length | RTL8139_TX_STATUS_OK);

    // Advance to next buffer
    dev->current_tx = (dev->current_tx + 1) % 4;

    return 0;
}

// -----------------------------------------------------------------------------
// Receive packet
// -----------------------------------------------------------------------------

int rtl8139_receive(rtl8139_device_t* dev, net_packet_t* packet) {
    if (!dev || !dev->initialized || !packet) {
        return -1;
    }

    // Check if packet is available
    uint16_t status = inw(dev->io_base + RTL8139_RX_BUF);
    if (!(status & 0x01)) {
        return -1;
    }

    // Read packet from receive buffer
    uint16_t length = (status >> 16) & 0xFFFF;
    if (length > 2048) {
        length = 2048;
    }

    for (uint16_t i = 0; i < length; i++) {
        packet->data[i] = inb(dev->io_base + RTL8139_RX_BUF + i);
    }

    packet->length = length;

    // Clear receive status
    outb(dev->io_base + RTL8139_INT_STATUS, RTL8139_INT_RX_OK);

    return 0;
}

// -----------------------------------------------------------------------------
// PCI driver for RTL8139
// -----------------------------------------------------------------------------

static pci_driver_t rtl8139_pci_driver = {
    .vendor_id = RTL8139_VENDOR_ID,
    .device_id = RTL8139_DEVICE_ID,
    .name = "RTL8139",
    .init = NULL
};

// -----------------------------------------------------------------------------
// Initialize network subsystem
// -----------------------------------------------------------------------------

void network_init(void) {
    vga_puts("Initializing network subsystem...\n");

    // Find RTL8139 device
    pci_device_t* device = pci_find_device(RTL8139_VENDOR_ID, RTL8139_DEVICE_ID);

    if (!device) {
        vga_puts("Network: RTL8139 not found\n");
        return;
    }

    // Set up device state
    memset(&rtl8139_dev, 0, sizeof(rtl8139_dev));
    rtl8139_dev.io_base = device->bar[0] & ~3;
    rtl8139_dev.irq = device->interrupt_line;

    // Register driver
    rtl8139_pci_driver.init = (int (*)(pci_device_t*))rtl8139_init;
    pci_register_driver(&rtl8139_pci_driver);

    vga_puts("Network: RTL8139 driver registered\n");
}
