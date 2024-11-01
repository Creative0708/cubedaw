/// Formatting helper to format a `&[u8]` as a Rust-like byte string.
pub struct ByteString<'a>(pub &'a [u8]);
impl std::fmt::Debug for ByteString<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        f.write_char('"')?;
        for &byte in self.0 {
            let should_show_as_hex = if f.alternate() {
                !(byte as char).is_ascii_graphic()
            } else {
                false
            };
            if should_show_as_hex {
                f.write_fmt(format_args!("\\x{byte:02x}"))?;
            } else {
                f.write_fmt(format_args!("{}", byte.escape_ascii()))?;
            }
        }
        f.write_char('"')?;
        Ok(())
    }
}
