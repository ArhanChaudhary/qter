use bitflags::bitflags;

/// The GCONF register address on the TMC2209.
///
/// See page 23 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const GCONF_REGISTER_ADDRESS: u8 = 0x00;
/// The IFCNT register address on the TMC2209.
///
/// See page 24 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const IFCNT_REGISTER_ADDRESS: u8 = 0x02;
/// The NODECONF register address on the TMC2209.
///
/// See page 24 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const NODECONF_REGISTER_ADDRESS: u8 = 0x03;
#[allow(clippy::doc_markdown)]
/// The IHOLD_IRUN register address on the TMC2209.
///
/// See page 28 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const IHOLD_IRUN_REGISTER_ADDRESS: u8 = 0x10;
/// The CHOPCONF register address on the TMC2209.
///
/// See page 33 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const CHOPCONF_REGISTER_ADDRESS: u8 = 0x6C;
#[allow(clippy::doc_markdown)]
/// The DRV_STATUS register address on the TMC2209.
///
/// See page 37 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const DRV_STATUS_REGISTER_ADDRESS: u8 = 0x6F;
/// The PWMCONF register address on the TMC2209.
///
/// See page 35 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
pub const PWMCONF_REGISTER_ADDRESS: u8 = 0x70;

bitflags! {
    /// The GCONF register bitflags on the TMC2209. UART is permitted to read
    /// and write to this register.
    ///
    /// See page 23 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct GCONF: u32 {
        /// Enable StealthChop (0) or SpreadCycle (1) mode.
        const EN_SPREADCYCLE = 1 << 2;
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

bitflags! {
    /// The NODECONF register bitflags on the TMC2209. UART is only permutted
    /// to write to this register, so all reads will return 0.
    #[derive(Debug, PartialEq, Clone, Copy)]
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
    /// The CHOPCONF register bitflags on the TMC2209. UART is permitted to read
    /// and write to this register.
    ///
    /// See page 33 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
    #[derive(Debug, PartialEq, Clone, Copy)]
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

bitflags! {
    /// The PWMCONF register bitflags on the TMC2209. UART is permitted to read
    /// and write to this register.
    ///
    /// See page 35 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct PWMCONF: u32 {
        /// Freewheel mode bit 0.
        const FREEWHEEL0 = 1 << 20;
        /// Freewheel mode bit 1.
        const FREEWHEEL1 = 1 << 21;

        /// ALl 32 bits are used.
        const _ = !0;
    }
}

bitflags! {
    /// The IHOLD_IRUN register bitflags on the TMC2209. UART is only permutted
    /// to write to this register, so all reads will return 0.
    ///
    /// See page 35 of <https://www.analog.com/media/en/technical-documentation/data-sheets/tmc2209_datasheet_rev1.09.pdf>
    #[derive(Debug, PartialEq, Clone, Copy)]
    #[allow(non_camel_case_types)]
    pub struct IHOLD_IRUN: u32 {
        /// IHOLD bit 0.
        const IHOLD0 = 1;
        /// IHOLD bit 1.
        const IHOLD1 = 1 << 1;
        /// IHOLD bit 2.
        const IHOLD2 = 1 << 2;
        /// IHOLD bit 3.
        const IHOLD3 = 1 << 3;
        /// IHOLD bit 4.
        const IHOLD4 = 1 << 4;
        /// IRUN bit 0.
        const IRUN0 = 1 << 8;
        /// IRUN bit 1.
        const IRUN1 = 1 << 9;
        /// IRUN bit 2.
        const IRUN2 = 1 << 10;
        /// IRUN bit 3.
        const IRUN3 = 1 << 11;
        /// IRUN bit 4.
        const IRUN4 = 1 << 12;
        /// IHOLDDELAY bit 0.
        const IHOLDDELAY0 = 1 << 16;
        /// IHOLDDELAY bit 1.
        const IHOLDDELAY1 = 1 << 17;
        /// IHOLDDELAY bit 2.
        const IHOLDDELAY2 = 1 << 18;
        /// IHOLDDELAY bit 3.
        const IHOLDDELAY3 = 1 << 19;

        /// Only the first 20 bits are used.
        const _ = 0b1111_0001_1111_0001_1111;
    }
}

bitflags! {
    /// The DRV_STATUS register bitflags on the TMC2209. UART is only permutted
    /// to read from this register.
    #[derive(Debug, PartialEq, Clone, Copy)]
    #[allow(non_camel_case_types)]
    pub struct DRV_STATUS: u32 {
        /// Overtemperature pre-warning flag
        const OTPW = 1 << 0;
        /// Overtemperature flag.
        const OT = 1 << 1;
        /// Short-to-ground on phase A.
        const S2GA = 1 << 2;
        /// Short-to-ground on phase B.
        const S2GB = 1 << 3;
        /// Low-side short on phase A.
        const S2VSA = 1 << 4;
        /// Low-side short on phase B.
        const S2VSB = 1 << 5;
        /// Open-load detected on phase A.
        const OLA = 1 << 6;
        /// Open-load detected on phase B.
        const OLB = 1 << 7;
        /// 120°C temperature threshold exceeded.
        const T120 = 1 << 8;
        /// 143°C temperature threshold exceeded.
        const T143 = 1 << 9;
        /// 150°C temperature threshold exceeded.
        const T150 = 1 << 10;
        /// 157°C temperature threshold exceeded.
        const T157 = 1 << 11;
        /// Bit 0 of the actual motor current.
        const CS_ACTUAL0 = 1 << 16;
        /// Bit 1 of the actual motor current.
        const CS_ACTUAL1 = 1 << 17;
        /// Bit 2 of the actual motor current.
        const CS_ACTUAL2 = 1 << 18;
        /// Bit 3 of the actual motor current.
        const CS_ACTUAL3 = 1 << 19;
        /// Bit 4 of the actual motor current.
        const CS_ACTUAL4 = 1 << 20;
        // All 32 bits are used.
        const _ = !0;
    }
}
