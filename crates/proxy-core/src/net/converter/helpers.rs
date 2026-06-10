//! Small per-converter utilities. Keep this file tiny — anything that
//! grows past a couple of helpers wants its own module.

use bytes::Bytes;

/// "Same packet, different id" path. Used when the only thing that
/// changed between versions is the packet id (e.g. when the wire
/// shape is identical but packet numbering shifted). The body bytes
/// are copied as-is.
pub fn rebuild_with_id(new_id: u8, body: &Bytes) -> crate::converter::ConversionResult {
    crate::converter::ConversionResult::Converted(vec![crate::converter::build_payload(
        new_id, body,
    )])
}
