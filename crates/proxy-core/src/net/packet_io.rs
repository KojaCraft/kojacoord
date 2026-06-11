//! Length-prefixed Minecraft framing with optional zlib compression.
//!
//! Below the per-version typed packets, every Minecraft packet on the
//! wire is `<length: varint><[data]>`. When compression is negotiated
//! the body becomes `<uncompressed_length: varint><compressed_data>`
//! and packets above the threshold are zlib-deflated. This module is
//! the read/write half of that — everything above (`connection.rs`,
//! `relay.rs`) hands frames in and out as `Bytes`.
//!
//! Framing mode by protocol epoch (see
//! [`kojacoord_protocol::Epoch`]):
//!   * [`Epoch::PreNetty`] (1.6.x) — no varint length prefix; the
//!     packet id is one raw byte and each packet has a static size
//!     determined by its id. Compression never applies. Callers that
//!     bridge a 1.6 client must speak the legacy framing directly via
//!     [`write_legacy_bytes`] / [`read_legacy_byte`] and rely on the
//!     v1_6_x typed packets for size.
//!   * Modern (1.7+) — `<length: varint><body>` with optional
//!     zlib compression once the threshold has been negotiated. Handled
//!     by [`read_frame`] / [`write_packet`].
//!
//! [`is_pre_netty_proto`] is a thin helper around `Epoch::PreNetty` so
//! call sites don't have to import the protocol crate just to gate on
//! the framing mode.

use std::io::{Read, Write};

use bytes::{BufMut, Bytes, BytesMut};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use kojacoord_protocol::{
    codec::Encode, types::VarInt, Decode, Epoch, ProtocolError, VersionRegistry,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::buffer_pool::GLOBAL_BUFFER_POOL;
use crate::error::ConnectionError;

pub const NO_COMPRESSION: i32 = -1;

pub const MAX_PACKET_SIZE: usize = 2 * 1024 * 1024;

/// True if the given negotiated protocol speaks the pre-netty wire
/// format (1.6.x and earlier). Pre-netty connections have no varint
/// length prefix and no compression layer — calling the modern
/// `read_frame` / `write_packet` helpers on one corrupts the stream.
pub fn is_pre_netty_proto(proto: u32) -> bool {
    VersionRegistry::nearest(proto).epoch() == Epoch::PreNetty
}

/// Write raw legacy bytes verbatim. Use only when speaking the
/// pre-netty framing (1.6.x) — no length prefix, no compression. The
/// caller has already laid out the packet id and body per the legacy
/// protocol spec.
pub async fn write_legacy_bytes<W: AsyncWriteExt + Unpin>(
    dst: &mut W,
    raw: &[u8],
) -> Result<(), ConnectionError> {
    dst.write_all(raw).await?;
    dst.flush().await?;
    Ok(())
}

/// Read a single byte from a pre-netty stream. Used to peek at the
/// next packet id; subsequent fields must be read directly by the
/// typed v1_6_x decoder since each packet has a static-known shape.
pub async fn read_legacy_byte<R: AsyncReadExt + Unpin>(src: &mut R) -> Result<u8, ConnectionError> {
    Ok(src.read_u8().await?)
}

/// Encode a single varint-length-prefixed frame. Modern framing only
/// (1.7+). Pre-netty callers must use [`write_legacy_bytes`].
pub fn encode_frame(body: &[u8]) -> BytesMut {
    let mut out = GLOBAL_BUFFER_POOL.acquire(5 + body.len());
    VarInt(body.len() as i32)
        .encode(&mut out)
        .expect("encoding a VarInt into a BytesMut never fails");
    out.put_slice(body);
    out
}

/// Read one varint-length-prefixed frame from `src`. Modern framing
/// only — see [`is_pre_netty_proto`].
pub async fn read_frame<R: AsyncReadExt + Unpin>(src: &mut R) -> Result<Bytes, ConnectionError> {
    let len = read_varint(src).await?;
    if len < 0 || len as usize > MAX_PACKET_SIZE {
        return Err(ConnectionError::Protocol(ProtocolError::PacketTooLarge(
            len as usize,
            MAX_PACKET_SIZE,
        )));
    }
    let mut body = GLOBAL_BUFFER_POOL.acquire(len as usize);
    body.resize(len as usize, 0);
    src.read_exact(&mut body).await?;
    Ok(body.freeze())
}

pub fn compress(raw: &[u8], threshold: i32) -> BytesMut {
    let mut out = GLOBAL_BUFFER_POOL.acquire(raw.len() + 5);
    if raw.len() >= threshold.max(0) as usize {
        VarInt(raw.len() as i32)
            .encode(&mut out)
            .expect("VarInt encode into BytesMut never fails");
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder
            .write_all(raw)
            .expect("zlib write into Vec never fails");
        let compressed = encoder.finish().expect("zlib finish into Vec never fails");
        out.put_slice(&compressed);
    } else {
        VarInt(0)
            .encode(&mut out)
            .expect("VarInt encode into BytesMut never fails");
        out.put_slice(raw);
    }
    out
}

pub fn decompress(body: Bytes) -> Result<Bytes, ConnectionError> {
    let mut cursor = body;
    let data_len = VarInt::decode(&mut cursor)
        .map_err(ConnectionError::Protocol)?
        .0;

    if data_len == 0 {
        return Ok(cursor);
    }
    if data_len < 0 || data_len as usize > MAX_PACKET_SIZE {
        return Err(ConnectionError::Protocol(ProtocolError::PacketTooLarge(
            data_len as usize,
            MAX_PACKET_SIZE,
        )));
    }

    let mut out = GLOBAL_BUFFER_POOL.acquire(data_len as usize);
    out.resize(data_len as usize, 0);
    ZlibDecoder::new(cursor.as_ref())
        .read_exact(&mut out)
        .map_err(ConnectionError::Io)?;
    Ok(out.freeze())
}

pub fn encode_packet(raw: &[u8], threshold: i32) -> BytesMut {
    if threshold >= 0 {
        let compressed = compress(raw, threshold);
        let frame = encode_frame(&compressed);
        GLOBAL_BUFFER_POOL.release(compressed);
        frame
    } else {
        encode_frame(raw)
    }
}

pub async fn read_packet<R: AsyncReadExt + Unpin>(
    src: &mut R,
    threshold: i32,
) -> Result<Bytes, ConnectionError> {
    let body = read_frame(src).await?;
    if threshold >= 0 {
        decompress(body)
    } else {
        Ok(body)
    }
}

pub async fn write_packet<W: AsyncWriteExt + Unpin>(
    dst: &mut W,
    raw: &[u8],
    threshold: i32,
) -> Result<(), ConnectionError> {
    let frame = encode_packet(raw, threshold);
    dst.write_all(&frame).await?;
    dst.flush().await?;
    GLOBAL_BUFFER_POOL.release(frame);
    Ok(())
}

/// Write a single typed play packet, choosing the correct framing for the
/// negotiated protocol version automatically.
///
/// * **Pre-netty (1.6.x):** raw bytes with no length prefix and no
///   compression. The packet id byte is prepended to the body and the whole
///   thing is handed to [`write_legacy_bytes`].
/// * **Modern (1.7+):** the packet id is varint-encoded and prepended to the
///   body, then the combined payload is varint-length-framed with optional
///   zlib compression via [`write_packet`].
///
/// `pid` must already be resolved by `PacketId::packet_id`. `body` is the
/// encoded packet body — everything *after* the id. The sentinel `0xFF`
/// (packet absent for this version) must be filtered out by the caller before
/// reaching here; this function does not check for it.
pub async fn write_typed_packet<W: AsyncWriteExt + Unpin>(
    dst: &mut W,
    pid: u8,
    body: &[u8],
    protocol_version: u32,
    compression_threshold: i32,
) -> Result<(), ConnectionError> {
    if is_pre_netty_proto(protocol_version) {
        // Pre-netty: one raw id byte followed by the body verbatim, no
        // varint framing, no compression.
        let mut raw = GLOBAL_BUFFER_POOL.acquire(1 + body.len());
        raw.put_u8(pid);
        raw.put_slice(body);
        let result = write_legacy_bytes(dst, &raw).await;
        GLOBAL_BUFFER_POOL.release(raw);
        result
    } else {
        // Modern: varint-encode the id, append the body, then hand the
        // combined payload to write_packet which applies framing and
        // optional zlib compression.
        let mut p = GLOBAL_BUFFER_POOL.acquire(5 + body.len());
        VarInt(pid as i32)
            .encode(&mut p)
            .expect("VarInt encode into BytesMut never fails");
        p.put_slice(body);
        let result = write_packet(dst, &p, compression_threshold).await;
        GLOBAL_BUFFER_POOL.release(p);
        result
    }
}

pub async fn read_varint<R: AsyncReadExt + Unpin>(src: &mut R) -> Result<i32, ConnectionError> {
    let mut result: u32 = 0;
    for i in 0..5 {
        let byte = src.read_u8().await?;
        result |= ((byte & 0x7F) as u32) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(result as i32);
        }
    }
    Err(ConnectionError::Protocol(ProtocolError::VarIntOverflow(5)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip_uncompressed() {
        let raw = b"\x00hello world";
        let frame = encode_packet(raw, NO_COMPRESSION);

        let mut cur = frame.freeze();
        let len = VarInt::decode(&mut cur).unwrap().0 as usize;
        assert_eq!(len, raw.len());
        assert_eq!(cur.as_ref(), raw);
    }

    #[tokio::test]
    async fn read_write_roundtrip_uncompressed() {
        let raw = b"\x17some packet body".to_vec();
        let frame = encode_packet(&raw, NO_COMPRESSION);
        let mut src = std::io::Cursor::new(frame.to_vec());
        let got = read_packet(&mut src, NO_COMPRESSION).await.unwrap();
        assert_eq!(got.as_ref(), raw.as_slice());
    }

    #[tokio::test]
    async fn read_write_roundtrip_compressed_large() {
        let raw: Vec<u8> = (0..1000u32).map(|i| (i % 7) as u8).collect();
        let frame = encode_packet(&raw, 256);
        let mut src = std::io::Cursor::new(frame.to_vec());
        let got = read_packet(&mut src, 256).await.unwrap();
        assert_eq!(got.as_ref(), raw.as_slice());
    }

    #[tokio::test]
    async fn read_write_roundtrip_compressed_below_threshold() {
        let raw = b"\x10tiny".to_vec();
        let frame = encode_packet(&raw, 256);

        let mut cur = frame.clone().freeze();
        let _frame_len = VarInt::decode(&mut cur).unwrap();
        let data_len = VarInt::decode(&mut cur).unwrap().0;
        assert_eq!(data_len, 0);

        let mut src = std::io::Cursor::new(frame.to_vec());
        let got = read_packet(&mut src, 256).await.unwrap();
        assert_eq!(got.as_ref(), raw.as_slice());
    }

    /// write_typed_packet (modern path) must produce a frame that
    /// read_packet can decode back to the original pid+body.
    #[tokio::test]
    async fn write_typed_packet_modern_roundtrip() {
        let pid: u8 = 0x26;
        let body = b"hello limbo";
        // proto 47 = 1.8, definitely modern
        let mut buf = Vec::new();
        write_typed_packet(&mut buf, pid, body, 47, NO_COMPRESSION)
            .await
            .unwrap();

        let mut src = std::io::Cursor::new(buf);
        let frame = read_packet(&mut src, NO_COMPRESSION).await.unwrap();
        // First byte(s) of frame are the varint-encoded pid.
        let mut cur = frame;
        let got_pid = VarInt::decode(&mut cur).unwrap().0 as u8;
        assert_eq!(got_pid, pid);
        assert_eq!(cur.as_ref(), body);
    }

    /// write_typed_packet (pre-netty path) must produce a raw id byte
    /// followed by the body with no framing wrapper.
    #[tokio::test]
    async fn write_typed_packet_pre_netty_roundtrip() {
        let pid: u8 = 0x01;
        let body = b"pre-netty body";
        // proto 78 = 1.6.4, pre-netty
        let mut buf = Vec::new();
        write_typed_packet(&mut buf, pid, body, 78, NO_COMPRESSION)
            .await
            .unwrap();

        assert_eq!(buf[0], pid);
        assert_eq!(&buf[1..], body);
    }
}
