// NebulaOS - Network Driver Header
// =================================
//
// Network device driver definitions

#ifndef NEBULAOS_DRIVERS_NETWORK_H
#define NEBULAOS_DRIVERS_NETWORK_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// RTL8139 PCI device ID
#define RTL8139_DEVICE_ID   0x8139
#define RTL8139_VENDOR_ID   0x10EC

// RTL8139 I/O port offsets
#define RTL8139_MAC0        0x00
#define RTL8139_MAR0        0x08
#define RTL8139_TX_STATUS0  0x10
#define RTL8139_TX_ADDR0    0x20
#define RTL8139_RX_BUF      0x20
#define RTL8139_RX_MODE     0xDA
#define RTL8139_INT_STATUS  0x3E
#define RTL8139_INT_MASK    0x3C
#define RTL8139_CONFIG1     0x43
#define RTL8139_CMD         0x37

// RTL8139 command register bits
#define RTL8139_CMD_RESET   (1 << 4)
#define RTL8139_CMD_RX_EN   (1 << 3)
#define RTL8139_CMD_TX_EN   (1 << 2)
#define RTL8139_CMD_BUS_MASTER (1 << 0)

// RTL8139 interrupt status bits
#define RTL8139_INT_SERR    (1 << 15)
#define RTL8139_INT_TIMEOUT (1 << 14)
#define RTL8139_INT_LEN_CHG (1 << 13)
#define RTL8139_INT_RX_OVERFLOW (1 << 4)
#define RTL8139_INT_TX_ERR  (1 << 5)
#define RTL8139_INT_TX_OK   (1 << 2)
#define RTL8139_INT_RX_OK   (1 << 0)

// RTL8139 receive mode bits
#define RTL8139_RX_ACCEPT_ERR   (1 << 0)
#define RTL8139_RX_ACCEPT_RUNT  (1 << 1)
#define RTL8139_RX_ACCEPT_BROADCAST (1 << 3)
#define RTL8139_RX_ACCEPT_MULTICAST (1 << 2)
#define RTL8139_RX_ACCEPT_PHYS_MATCH (1 << 5)
#define RTL8139_RX_ACCEPT_ALL (1 << 7)

// RTL8139 TX status bits
#define RTL8139_TX_STATUS_OWNED   (1 << 13)
#define RTL8139_TX_STATUS_OK      (1 << 15)
#define RTL8139_TX_STATUS_UNDERRUN (1 << 14)

// RTL8139 transmit buffer sizes
#define RTL8139_TX_BUF_SIZE 1536

// Network packet buffer
typedef struct {
    uint8_t data[2048];
    uint16_t length;
} net_packet_t;

// RTL8139 device state
typedef struct {
    uint16_t io_base;
    uint8_t  irq;
    uint8_t  mac[6];
    bool     initialized;
    uint8_t* rx_buffer;
    uint8_t* tx_buffers[4];
    uint32_t tx_offsets[4];
    uint32_t current_tx;
} rtl8139_device_t;

// Network driver interface
typedef struct {
    const char* name;
    uint16_t vendor_id;
    uint16_t device_id;
    int (*init)(rtl8139_device_t* dev);
    int (*send)(rtl8139_device_t* dev, const uint8_t* data, uint16_t length);
    int (*receive)(rtl8139_device_t* dev, net_packet_t* packet);
} network_driver_t;

// Network functions
void network_init(void);
int rtl8139_init(rtl8139_device_t* dev);
int rtl8139_send(rtl8139_device_t* dev, const uint8_t* data, uint16_t length);
int rtl8139_receive(rtl8139_device_t* dev, net_packet_t* packet);
void rtl8139_reset(rtl8139_device_t* dev);

// Helper functions
void rtl8139_read_mac(rtl8139_device_t* dev);
void rtl8139_enable_interrupts(rtl8139_device_t* dev);
void rtl8139_disable_interrupts(rtl8139_device_t* dev);
void rtl8139_start_receiver(rtl8139_device_t* dev);
void rtl8139_stop_receiver(rtl8139_device_t* dev);

#endif // NEBULAOS_DRIVERS_NETWORK_H
