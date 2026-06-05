use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("unexpected end of packet data")]
    UnexpectedEof,

    #[error("variable-length integer overflow (read {0} bytes)")]
    VarIntOverflow(usize),

    #[error("string length {0} exceeds maximum {1}")]
    StringTooLong(usize, usize),

    #[error("string contains invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("unknown packet id 0x{id:02X} in state {state:?} direction {direction:?}")]
    UnknownPacketId {
        id: u32,

        state: String,

        direction: String,
    },

    #[error("unrecognized NBT tag type: {0}")]
    UnknownNbtTag(u8),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("packet size {0} exceeds maximum {1}")]
    PacketTooLarge(usize, usize),
}
