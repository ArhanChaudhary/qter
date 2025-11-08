use std::time::Duration;

use bitflags::bitflags;
use rppal::uart::{Parity, Uart};

// Can be anywhere from 9,000 to 500,000?
// Keep in mind it also needs to be equal to one of the allowed values on the raspi
const BAUD_RATE: u32 = 460_800;

pub fn mk_uart(path: &str) -> Uart {
    Uart::with_path(path, BAUD_RATE, Parity::None, 8, 1).unwrap()
}

fn write_packet(address: u8, register: u8, val: u32) -> [u8; 8] {
    assert!(address < 4);
    assert!(register & 0x80 == 0);
    let val = val.to_be_bytes();

    let mut out = [
        0b_0000_0101,
        address,
        register | 0x80,
        val[0],
        val[1],
        val[2],
        val[3],
        0,
    ];
    set_crc(&mut out);
    out
}

fn read_packet(address: u8, register: u8) -> [u8; 4] {
    assert!(address < 4);
    assert!(register & 0x80 == 0);

    let mut out = [0b_0000_0101, address, register, 0];
    set_crc(&mut out);
    out
}

// copied and adapted from datasheet
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

fn set_crc<const N: usize>(data: &mut [u8; N]) {
    data[N - 1] = calc_crc(data);
}

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

pub fn write(uart: &mut Uart, address: u8, register: u8, val: u32) {
    let packet = write_packet(address, register, val);
    uart.set_write_mode(true).unwrap();
    eprint!(
        "TX [0b{:08b}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}]...",
        packet[0], packet[1], packet[2], packet[3], packet[4], packet[5], packet[6], packet[7],
    );
    uart.write(&packet).unwrap();
    eprintln!(" done.");
}

fn read_(uart: &mut Uart) -> (u8, u8, Option<u32>) {
    let mut buf = [0; 4];
    uart.set_read_mode(4, Duration::ZERO).unwrap();
    eprint!("RX...");
    uart.read(&mut buf).unwrap();
    eprintln!(
        " done: [0b{:08b}, 0x{:02x}, 0x{:02x}, 0x{:02x}]",
        buf[0], buf[1], buf[2], buf[3]
    );
    let address = buf[1];
    let register = buf[2] & !0x80;
    let has_data = buf[2] & 0x80 > 0 || address == 0xff;
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

pub fn read(uart: &mut Uart, address: u8, register: u8) -> u32 {
    let packet = read_packet(address, register);
    uart.set_write_mode(true).unwrap();
    eprint!(
        "TX [0b{:08b}, 0x{:02x}, 0x{:02x}, 0x{:02x}]...",
        packet[0], packet[1], packet[2], packet[3]
    );
    uart.write(&packet).unwrap();
    eprintln!(" done.");

    loop {
        let (address2, register2, val) = read_(uart);
        if address2 == 0xFF
            && register2 == register
            && let Some(val) = val
        {
            break val;
        }
    }
}
