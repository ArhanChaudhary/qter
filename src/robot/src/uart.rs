use crate::WhichUart;
use bitflags::bitflags;
use log::debug;
use rppal::uart::{Parity, Uart};
use std::{thread, time::Duration};

/// Baud rates from 9600 to 500000 may be used. No baud rate configuration is
/// required, as the TMC2209 automatically adapts to the masters’ baud rate.
/// Keep in mind it also needs to be equal to one of the allowed values on the
/// Raspberry Pi 4 Model B. As such the only allowed baud rates are `9_600`,
/// `19_200`, `38_400`, `57_600`, `115_200`, `230_400`, `460_800`. Generally
/// higher is better, so we set it to just below `460_800` for safety margin.
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
/// The size of the read access reply is 8 bytes, so one might think we should
/// read 8 bytes at a time from UART. However, our multi-node setup connects
/// RX and TX through a 1kΩ resistor, which causes the TMC2209 to immediately
/// send back the 4 byte read packet. Thus we only need to read 4 bytes at a
/// time.
///
/// See page 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_READ_BUFFER_SIZE_BYTES: u8 = 4;
/// The UART sync byte used to identify the start of all UART transmissions.
///
/// See page 18 & 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const UART_SYNC_BYTE: u8 = 0b_1010_0000u8.reverse_bits();
/// The bit mask of the read/write bit in the UART register address for all UART
/// transmissions.
///
/// See page 18 & 19 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
const REGISTER_MSB: u8 = 1 << 7;

// 0x0, n = 10, RW
bitflags! {
    #[derive(Debug)]
    pub struct GConf: u32 {
        const SHAFT = 1 << 3;
        const INDEX_OTPW = 1 << 4;
        const PDN_DISABLE = 1 << 6;
        const MSTEP_REG_SELECT = 1 << 7;

        const _ = (1 << 10) - 1;
    }
}

// 0x1, n = 3, R+WC
bitflags! {
    #[derive(Debug)]
    pub struct GStat: u32 {
        const RESET = 1 << 0;
        const DRV_ERR = 1 << 1;
        const UV_CP = 1 << 2;
    }
}

// read IFCNT from 0x2, n = 8

/// Create a new `Uart`.
pub fn mk_uart(which_uart: WhichUart) -> Uart {
    // uart0 is /dev/ttyAMA0 and uart2 is /dev/ttyAMA1 on Raspberry Pi 4 Model B.
    //
    // See https://forums.raspberrypi.com/viewtopic.php?t=244827#post_content1514245
    let path = match which_uart {
        WhichUart::Uart0 => "/dev/ttyAMA0",
        WhichUart::Uart2 => "/dev/ttyAMA1",
    };

    debug!(target: "uart", "action=mk_uart path={path} baud={UART_BAUD_RATE}");

    let mut uart = Uart::with_path(
        path,
        UART_BAUD_RATE,
        UART_PARITY,
        UART_DATA_BITS,
        UART_STOP_BITS,
    )
    .unwrap();

    // TODO: we want reads and writes to be non-blocking.
    uart.set_read_mode(UART_READ_BUFFER_SIZE_BYTES, Duration::ZERO)
        .unwrap();
    uart.set_write_mode(true).unwrap();

    debug!(target: "uart", "action=mk_uart_complete path={path}");

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
    assert!(node_address < 4);
    assert!(register_address & REGISTER_MSB == 0);

    let mut read_packet = [UART_SYNC_BYTE, node_address, register_address, 0];
    read_packet[3] = calc_crc(&read_packet);

    debug!(
        target: "uart",
        "action=mk_read_packet node={node_address} register_address={register_address} packet=[0b{:08b},0x{:02x},0x{:02x},0x{:02x}]",
        read_packet[0],
        read_packet[1],
        read_packet[2],
        read_packet[3]
    );

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
/// A = TMC2209 node address (0-3)
/// R = register address (0-127)
/// D = data bytes
/// C = CRC
///
/// See page 18 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
fn mk_write_packet(node_address: u8, register_address: u8, val: u32) -> [u8; 8] {
    assert!(node_address < 4);
    assert!(register_address & REGISTER_MSB == 0);

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
        "action=mk_write_packet node={node_address} register_address={register_address} value=0x{val:08x} packet=[0b{:08b},0x{:02x},0x{:02x},0x{:02x},0x{:02x},0x{:02x},0x{:02x},0x{:02x}]",
        write_packet[0],
        write_packet[1],
        write_packet[2],
        write_packet[3],
        write_packet[4],
        write_packet[5],
        write_packet[6],
        write_packet[7]
    );

    write_packet
}

/// Read a register address via UART for a given node address.
///
/// This sends a 4-byte read packet then waits for the appropriate reply.
/// Returns the 32-bit data payload.
pub fn read(uart: &mut Uart, node_address: u8, register_address: u8) -> u32 {
    let read_packet = mk_read_packet(node_address, register_address);
    // TODO: the stepper driver needs a small delay between UART operations, for now i just
    //       sleep for 1ms but eventually this should be integrated into the actual UART code
    thread::sleep(UART_DELAY);
    debug!(
        target: "uart",
        "action=tx_read_packet_sending node={} register_address={}",
        read_packet[1],
        read_packet[2]
    );
    debug!(target: "uart", "");
    uart.write(&read_packet).unwrap();

    debug!(
        target: "uart",
        "action=tx_read_packet_complete node={} register_address={}",
        read_packet[1],
        read_packet[2]
    );

    loop {
        let (reply_node_address, reply_register_address, maybe_data) = read_raw(uart);
        debug!(
            target: "uart",
            "action=rx_reply_header node=0x{reply_node_address:02x} register_address=0x{reply_register_address:02x} has_data={}",
            maybe_data.is_some()
        );

        if reply_node_address == 0xFF
            && reply_register_address == register_address
            && let Some(data) = maybe_data
        {
            debug!(
                target: "uart",
                "action=read_reply node=0xFF register_address=0x{reply_node_address:02x} data=0x{data:08x}",
            );
            break data;
        }
    }
}

/// Read from UART and return the TMC node address, register address, and
/// payload.
///
/// The 64-bit UART read access reply packet is specified as follows:
///
/// 1010----
/// 11111111
/// RRRRRRR0
/// DDDDDDDD
/// DDDDDDDD
/// DDDDDDDD
/// DDDDDDDD
/// CCCCCCCC
///
/// `-` = unused (0)
/// R = register address (0-127)
/// D = data bytes
/// C = CRC
fn read_raw(uart: &mut Uart) -> (u8, u8, Option<u32>) {
    debug!(target: "uart", "action=rx_read_header expecting=4_bytes");
    debug!(target: "uart", "");
    let mut half_read_buf = [0u8; 4];
    uart.read(&mut half_read_buf).unwrap();
    debug!(
        target: "uart",
        "action=rx_header_received bytes=[0b{:08b},0x{:02x},0x{:02x},0x{:02x}]",
        half_read_buf[0],
        half_read_buf[1],
        half_read_buf[2],
        half_read_buf[3]
    );

    let node_address = half_read_buf[1];
    let register_address = half_read_buf[2] & !REGISTER_MSB;
    let has_data = half_read_buf[2] & REGISTER_MSB != 0 || node_address == 0xff;

    let maybe_data = if has_data {
        let mut full_read_buf = {
            let mut full_read_buf = [0u8; 8];
            full_read_buf[0..4].copy_from_slice(&half_read_buf);
            full_read_buf
        };

        debug!(
            target: "uart",
            "action=rx_read_payload expecting=4_more_bytes node={node_address} register_address={register_address}",
        );
        debug!(target: "uart", "");
        uart.read(&mut full_read_buf[4..8]).unwrap();
        debug!(
            target: "uart",
            "action=rx_payload_received bytes=[0x{:02x},0x{:02x},0x{:02x},0x{:02x}] crc=0x{:02x}",
            full_read_buf[4],
            full_read_buf[5],
            full_read_buf[6],
            full_read_buf[7],
            full_read_buf[7]
        );

        let data_val = u32::from_be_bytes(full_read_buf[3..7].try_into().unwrap());
        let crc = full_read_buf[7];
        let expected_crc = calc_crc(&full_read_buf);
        assert_eq!(crc, expected_crc, "UART CRC mismatch");

        debug!(
            target: "uart",
            "action=rx_payload_validated node={node_address} register_address={register_address} crc=0x{crc:02x}",
        );

        Some(data_val)
    } else {
        let crc = half_read_buf[3];
        let expected_crc = calc_crc(&half_read_buf);
        assert_eq!(
            crc, expected_crc,
            "UART CRC mismatch for the read packet sendback"
        );
        debug!(
            target: "uart",
            "action=rx_read_sendback_validated node={node_address} register_address={register_address} crc=0x{crc:02x}",
        );
        None
    };

    (node_address, register_address, maybe_data)
}

/// Write to a register address through UART given a TMC2209 node address.
pub fn write(uart: &mut Uart, node_address: u8, register_address: u8, val: u32) {
    let packet = mk_write_packet(node_address, register_address, val);
    // TODO: the stepper driver needs a small delay between uart operations, for now i just
    //       sleep for 1ms but eventually this should be integrated into the actual uart code
    thread::sleep(UART_DELAY);
    debug!(
        target: "uart",
        "action=tx_write_packet_sending node=0x{:02x} register_address=0x{:02x}",
        packet[1],
        packet[2]
    );
    debug!(target: "uart", "");
    uart.write(&packet).unwrap();

    debug!(
        target: "uart",
        "action=tx_write_packet_complete node=0x{:02x} register_address=0x{:02x} crc=0x{:02x}",
        packet[1],
        packet[2],
        packet[7]
    );
}

/// Copied and adapted from page 20 of
/// <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
fn calc_crc<const N: usize>(data: &[u8; N]) -> u8 {
    let mut crc = 0u8;
    for mut current_byte in data.iter().take(N - 1).copied() {
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
