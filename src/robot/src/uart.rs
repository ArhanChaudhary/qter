use bitflags::bitflags;
use rppal::uart::{Parity, Uart};
use std::time::Duration;

// Can be anywhere from 9,000 to 500,000?
// Keep in mind it also needs to be equal to one of the allowed values on the
// Raspberry Pi 4 Model B
const BAUD_RATE: u32 = 460_800;
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

/// Create a new `Uart` from a device path.
pub fn mk_uart(path: &str) -> Uart {
    let uart = Uart::with_path(
        path,
        BAUD_RATE,
        // omit the parity bit
        Parity::None,
        // transfer 8 bits at a time
        8,
        // TMC2209 ends transmission with a single stop bit
        1,
    )
    .unwrap();
    // for each subsequent read, read and block until 4 bytes (size of read
    // packet) are available
    // TODO: we want reads to be non-blocking
    // uart.set_read_mode(4, Duration::ZERO).unwrap();
    uart
}

/// The 32-bit UART read packet is specified as follows:
///
/// 1010----
/// AA------
/// DDDDDDDD
/// CCCCCCCC
///
/// - = unused (0)
/// A = TMC2209 node address (0-3)
/// D = data bytes
/// C = CRC
///
/// See page 19 of https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf
fn mk_read_packet(address: u8, register: u8) -> [u8; 4] {
    // we only have three TMCs connected
    assert!(address < 3);
    // this bit must be zero
    assert!(register & REGISTER_MSB == 0);

    let mut out = [0b_1010_0000u8.reverse_bits(), address, register, 0];
    out[3] = calc_crc(&out);
    out
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
/// - = unused (0)
/// A = TMC2209 node address (0-3)
/// R = register (0-127)
/// D = data bytes
/// C = CRC
///
/// See page 18 of https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf
fn mk_write_packet(address: u8, register: u8, val: u32) -> [u8; 8] {
    // we only have three TMCs connected
    assert!(address < 3);
    // this bit must be one
    assert!(register & REGISTER_MSB == 0);

    let val = val.to_be_bytes();
    let mut out = [
        0b_1010_0000u8.reverse_bits(),
        address,
        register | REGISTER_MSB,
        val[0],
        val[1],
        val[2],
        val[3],
        0,
    ];
    out[7] = calc_crc(&out);
    out
}

/// Read a register through UART given a TMC2209 node address.
pub fn read(uart: &mut Uart, address: u8, register: u8) -> u32 {
    let read_packet = mk_read_packet(address, register);
    // TODO: we need to make it non-blocking
    uart.set_write_mode(true).unwrap();
    eprint!(
        "TX [0b{:08b}, 0x{:02x}, 0x{:02x}, 0x{:02x}]...",
        read_packet[0], read_packet[1], read_packet[2], read_packet[3]
    );
    uart.write(&read_packet).unwrap();
    eprintln!(" sent read packet.");

    loop {
        let (address2, register2, val) = read_raw(uart);
        if address2 == 0xFF
            && register2 == register
            && let Some(val) = val
        {
            break val;
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
/// - = unused (0)
/// R = register (0-127)
/// D = data bytes
/// C = CRC
fn read_raw(uart: &mut Uart) -> (u8, u8, Option<u32>) {
    uart.set_read_mode(4, Duration::ZERO).unwrap();
    
    eprint!(" reading.");
    let mut buf = [0; 4];
    uart.read(&mut buf).unwrap();
    eprintln!(
        " RX: [0b{:08b}, 0x{:02x}, 0x{:02x}, 0x{:02x}]",
        buf[0], buf[1], buf[2], buf[3]
    );
    
    let address = buf[1];
    let register = buf[2] & !REGISTER_MSB;
    let has_data = buf[2] & REGISTER_MSB != 0 || address == 0xff;
    let data = if has_data {
        let mut buf = {
            let mut new_buf = [0; 8];
            new_buf[0..4].copy_from_slice(&buf);
            new_buf
        };
        eprint!("RX...");
        uart.read(&mut buf[4..8]).unwrap();
        eprintln!(
            " done: [0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}]",
            buf[4], buf[5], buf[6], buf[7]
        );

        let data = u32::from_be_bytes(buf[3..7].try_into().unwrap());
        let crc = buf[7];
        let expected_crc = calc_crc(&buf);
        assert_eq!(crc, expected_crc);
        Some(data)
    } else {
        let crc = buf[3];
        let expected_crc = calc_crc(&buf);
        assert_eq!(crc, expected_crc);
        None
    };
    (address, register, data)
}

/// Write to a register through UART given a TMC2209 node address.
pub fn write(uart: &mut Uart, address: u8, register: u8, val: u32) {
    let packet = mk_write_packet(address, register, val);
    uart.set_write_mode(true).unwrap();
    eprint!(
        "TX [0b{:08b}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}]...",
        packet[0], packet[1], packet[2], packet[3], packet[4], packet[5], packet[6], packet[7],
    );
    uart.write(&packet).unwrap();
    eprintln!(" done.");
}

/// Copied and adapted from page 20 of
/// https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf
fn calc_crc<const N: usize>(data: &[u8; N]) -> u8 {
    let mut crc = 0u8;
    for i in 0..N - 1 {
        let mut current_byte = data[i];
        for _ in 0..8 {
            if (crc >> 7) ^ (current_byte & 0x01) > 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc = crc << 1;
            }
            current_byte >>= 1;
        }
    }
    crc
}