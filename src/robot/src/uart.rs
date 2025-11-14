use crate::WhichUart;
use bitflags::bitflags;
use log::{debug, trace};
use rppal::uart::{Parity, Uart};
use std::{thread, time::Duration};

/// Baud rates from 9600 to 500000 may be used. No baud rate configuration is
/// required, as the TMC2209 automatically adapts to the masters’ baud rate.
/// Keep in mind it also needs to be equal to one of the allowed values on the
/// Raspberry Pi 4 Model B. As such the only allowed baud rates are `9_600`,
/// `19_200`, `38_400`, `57_600`, `115_200`, `230_400`, `460_800` or `500_000`.
/// Generally higher is better, so we set it to `460_800` for safety margin.
///
/// See page 6 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf> and
/// <https://docs.golemparts.com/rppal/0.14.1/src/rppal/uart.rs.html#527>
const UART_BAUD_RATE: u32 = 460_800;
/// In a multi-node setup, UART has no parity.
///
/// See page 21 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_PARITY: Parity = Parity::None;
/// In a multi-node setup, UART uses 8 data bits.
///
/// See page 21 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_DATA_BITS: u8 = 8;
/// UART uses 1 stop bit.
///
/// See page 18 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_STOP_BITS: u8 = 1;
/// The TMC2209 datasheet specifies a small delay between UART operations, but
/// does not elaborate. 1ms should be enough to cover all, so we use it for now.
const UART_DELAY: Duration = Duration::from_millis(1);
/// For now our reads are blocking. We must provide a minimum UART read buffer
/// size, which is 8 bytes or the size of the read access reply.
///
/// See page 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_READ_BUFFER_SIZE_BYTES: u8 = 8;
/// The UART sync byte used to identify the start of all UART transmissions.
///
/// See page 18 & 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_SYNC_BYTE: u8 = 0b_1010_0000u8.reverse_bits();
/// The UART read access reply master address.
///
/// See page 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_READ_REPLY_MASTER_ADDRESS: u8 = 0xFF;
/// The bit mask of the read/write bit in the UART register address for all UART
/// transmissions.
///
/// See page 18 & 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const REGISTER_MSB: u8 = 1 << 7;
/// The GCONF register address on the TMC2209.
/// 
/// See page 23 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const GCONF_REGISTER_ADDRESS: u8 = 0x00;
/// The NODECONF register address on the TMC2209.
/// 
/// See page 24 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const NODECONF_REGISTER_ADDRESS: u8 = 0x03;
/// The CHOPCONF register address on the TMC2209.
/// 
/// See page 33 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const CHOPCONF_REGISTER_ADDRESS: u8 = 0x6C;

bitflags! {
    /// The GCONF register bitflags on the TMC2209. UART is permitted to read
    /// and write to this register.
    ///
    /// See page 23 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct GCONF: u32 {
        /// Inverse the motor direction.
        const SHAFT = 1 << 3;
        // INDEX pin outputs overtemperature prewarning flag (otpw)
        const INDEX_OTPW = 1 << 4;
        // PDN_UART input function disabled; set to use UART
        const PDN_DISABLE = 1 << 6;
        // Microstep resolution selected by MRES register
        const MSTEP_REG_SELECT = 1 << 7;

        /// Only the first ten bits are used.
        const _ = (1 << 10) - 1;
    }
}

// I can't see us needing this in the near future...
// 0x1, n = 3, R+WC
bitflags! {
    #[derive(Debug)]
    pub struct GSTAT: u32 {
        const RESET = 1 << 0;
        const DRV_ERR = 1 << 1;
        const UV_CP = 1 << 2;
    }
}

// read IFCNT from 0x2, n = 8

bitflags! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    /// The NODECONF register bitflags on the TMC2209. UART is only permutted
    /// to write to this register, so all reads will return 0.
    pub struct NODECONF: u32 {
        /// SENDDELAY bit 0.
        const SENDDELAY0 = 1;
        /// SENDDELAY bit 1.
        const SENDDELAY1 = 1 << 1;
        /// SENDDELAY bit 2.
        const SENDDELAY2 = 1 << 2;
        /// SENDDELAY bit 3.
        const SENDDELAY3 = 1 << 3;
        
        // Only the first four bits are used.
        const _ = (1 << 4) - 1;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    /// The CHOPCONF register bitflags on the TMC2209. UART is permitted to read
    /// and write to this register.
    ///
    /// See page 33 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
    pub struct CHOPCONF: u32 {
        /// Microstep resolution bit 0.
        const MRES0 = 1 << 24;
        /// Microstep resolution bit 1.
        const MRES1 = 1 << 25;
        /// Microstep resolution bit 2.
        const MRES2 = 1 << 26;
        /// Microstep resolution bit 3.
        const MRES3 = 1 << 27;
        
        /// ALl 32 bits are used.
        const _ = !0;
    }
}

/// Create a new `Uart`.
pub fn mk_uart(which_uart: WhichUart) -> Uart {
    // uart0 is /dev/ttyAMA0 and uart2 is /dev/ttyAMA1 on Raspberry Pi 4 Model B.
    //
    // See https://forums.raspberrypi.com/viewtopic.php?t=244827#post_content1514245
    let path = match which_uart {
        WhichUart::Uart0 => "/dev/ttyAMA0",
        WhichUart::Uart2 => "/dev/ttyAMA1",
    };

    debug!(target: "uart", "Initializing {which_uart:?}: path={path} baud_rate={UART_BAUD_RATE}");

    let mut uart = Uart::with_path(
        path,
        UART_BAUD_RATE,
        UART_PARITY,
        UART_DATA_BITS,
        UART_STOP_BITS,
    )
    .expect("Failed to initialize UART");
    
    // For now we use blocking reads and writes.
    uart.set_read_mode(UART_READ_BUFFER_SIZE_BYTES, Duration::ZERO)
        .expect("Failed to set UART read mode");
    uart.set_write_mode(true)
        .expect("Failed to set UART write mode");

    debug!(target: "uart", "Initialized {which_uart:?}");

    uart
}

/// The 32-bit UART read packet is specified as follows:
///
/// 1010----
/// AA------
/// RRRRRRR0
/// CCCCCCCC
///
/// `-` = unused (0)
/// A = TMC2209 node address (0-3)
/// R = register address (0-127)
/// C = CRC
///
/// See page 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
fn mk_read_packet(node_address: u8, register_address: u8) -> [u8; 4] {
    assert!(node_address < 4, "Node address must be in 0-4");
    assert_eq!(
        register_address & REGISTER_MSB,
        0,
        "Register address MSB must be 0"
    );

    let mut read_packet = [UART_SYNC_BYTE, node_address, register_address, 0];
    read_packet[3] = calc_crc(&read_packet);

    debug!(
        target: "uart",
        "Created read packet: node_adress={node_address} register_address={register_address}"
    );
    trace!("crc=0x{:02x}", read_packet[3]);

    read_packet
}

/// The 64-bit UART write packet is specified as follows:
///
/// 1010----
/// AA------
/// RRRRRRR1
/// DDDDDDDD
/// DDDDDDDD
/// DDDDDDDD
/// DDDDDDDD
/// CCCCCCCC
///
/// `-` = unused (0)
/// A = TMC2209 node address (0-4)
/// R = register address (0-127)
/// D = data bytes
/// C = CRC
///
/// See page 18 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
fn mk_write_packet(node_address: u8, register_address: u8, val: u32) -> [u8; 8] {
    assert!(node_address < 4, "Node address must be in 0-4");
    assert_eq!(
        register_address & REGISTER_MSB,
        0,
        "Register address MSB must be 0"
    );

    let val_bytes = val.to_be_bytes();
    let mut write_packet = [
        UART_SYNC_BYTE,
        node_address,
        register_address | REGISTER_MSB,
        val_bytes[0],
        val_bytes[1],
        val_bytes[2],
        val_bytes[3],
        0,
    ];
    write_packet[7] = calc_crc(&write_packet);

    debug!(
        target: "uart",
        "Created write packet: node_address={node_address} register_address={register_address} val=0x{val:08x}",
    );
    trace!(target: "uart", "crc=0x{:02x}", write_packet[7]);

    write_packet
}

/// Read a register address via UART for a given node address. First waits for
/// `UART_DELAY`.
pub fn read(uart: &mut Uart, node_address: u8, register_address: u8) -> u32 {
    debug!(target: "uart", "Reading register");
    let read_packet = mk_read_packet(node_address, register_address);
    debug!(target: "uart", "Sleeping before send");
    thread::sleep(UART_DELAY);
    send_packet(uart, read_packet);

    let mut read_reply_packet = [0; 8];
    debug!(target: "uart", "Sleeping before recv");
    thread::sleep(UART_DELAY);
    recv_packet(uart, &mut read_reply_packet);
    // The 64-bit UART read access reply packet is specified as follows:
    //
    // 1010----
    // 11111111
    // RRRRRRR0
    // DDDDDDDD
    // DDDDDDDD
    // DDDDDDDD
    // DDDDDDDD
    // CCCCCCCC
    //
    // `-` = unused (0)
    // R = register address (0-127)
    // D = data bytes
    // C = CRC
    let reply_master_address = read_reply_packet[1];
    let reply_register_address = read_reply_packet[2];
    assert_eq!(
        reply_register_address & REGISTER_MSB,
        0,
        "UART read reply register address MSB must be 0"
    );
    let data = u32::from_be_bytes(read_reply_packet[3..7].try_into().unwrap());
    let crc = read_reply_packet[7];

    let expected_crc = calc_crc(&read_reply_packet);
    assert_eq!(crc, expected_crc, "UART CRC mismatch");
    assert_eq!(
        reply_master_address, UART_READ_REPLY_MASTER_ADDRESS,
        "UART read reply master address mismatch"
    );
    assert_eq!(
        reply_register_address, register_address,
        "UART read reply register address mismatch"
    );
    debug!(
        target: "uart",
        "Received reply packet"
    );
    trace!(
        target: "uart",
        "crc=0x{crc:02x}"
    );

    data
}

/// Write to a register address through UART given a TMC2209 node address. First
/// waits for `UART_DELAY`.
pub fn write(uart: &mut Uart, node_address: u8, register_address: u8, val: u32) {
    debug!(target: "uart", "Writing to register");
    let write_packet = mk_write_packet(node_address, register_address, val);
    debug!(target: "uart", "Sleeping before send");
    thread::sleep(UART_DELAY);
    send_packet(uart, write_packet);
}

/// Receive a packet via UART into the provided buffer.
fn recv_packet(uart: &mut Uart, packet: &mut [u8]) {
    debug!(target: "transmission", "Receiving packet");
    let written = uart.read(packet).unwrap();
    assert_eq!(written, packet.len(), "UART read packet size mismatch");
    debug!(target: "transmission", "Received packet");
}

/// Send a packet via UART and verify the sendback.
fn send_packet<const N: usize>(uart: &mut Uart, packet: [u8; N]) {
    debug!(target: "transmission", "Sending packet");
    let written = uart.write(&packet).unwrap();
    assert_eq!(written, N, "UART write packet size mismatch");
    debug!(target: "transmission", "Sent packet");

    let mut sendback_packet = [0; N];
    debug!(target: "transmission", "Receiving sendback packet");
    let written = uart.read(&mut sendback_packet).unwrap();
    assert_eq!(written, N, "UART sendback packet size mismatch");
    debug!(target: "transmission", "Received sendback packet");

    assert_eq!(packet, sendback_packet, "UART sendback packet mismatch");
    debug!(target: "transmission", "Verified sendback packet");
}

/// Copied and adapted from page 20 of
/// <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
fn calc_crc(packet: &[u8]) -> u8 {
    let mut crc = 0;
    for mut current_byte in packet.iter().take(packet.len() - 1).copied() {
        for _ in 0..8 {
            if (crc >> 7) ^ (current_byte & 0x01) > 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
            current_byte >>= 1;
        }
    }
    crc
}
