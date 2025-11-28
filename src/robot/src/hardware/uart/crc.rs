/// Calculates the CRC for the given packet, excluding the last byte
/// (because the last byte of the packet is where the CRC goes).
///
/// Copied and adapted from datasheet pg. 20.
pub fn calc_crc(packet: &[u8]) -> u8 {
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

/// Returns the given array, with the last byte set to the CRC.
pub fn with_crc<const N: usize>(mut packet: [u8; N]) -> [u8; N] {
    packet[N - 1] = calc_crc(&packet);
    packet
}
