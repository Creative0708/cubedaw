
// Number serialization

use std::io::{Write, Read};
use super::{Serialize, Result, util::invalid_data};
use super::prelude::*;

impl Serialize for u64 {
    fn serialize(&self, writer: &mut impl Write) -> Result<()>{
        let mut val = *self;

        if val <= 127 {
            writer.write(&[val as u8])?;
            return Ok(());
        }

        let mut slice: [u8; 10] = [0; 10];
        let mut current_index = 0;
        loop {
            slice[current_index] = (val & 0x7f) as u8;
            val >>= 7;
            if val == 0 {
                break;
            }
            slice[current_index] |= 0x80;
            current_index += 1;
        }

        writer.write_all(&slice[0..=current_index])?;

        Ok(())
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let mut val: u64 = 0;
        let mut shift: u8 = 0;

        for byte in reader.bytes() {
            let byte = byte?;
            val |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 64 {
                return Err(invalid_data!(""));
            }
        }

        Ok(val)
    }
}
impl Serialize for i64 {
    fn serialize(&self, writer: &mut impl Write) -> Result<()>{
        let mut val = *self;

        if val >= -128 && val <= 127 {
            writer.write(&[(val as u8) & 0x7f])?;
            return Ok(());
        }

        let mut slice: [u8; 10] = [0; 10];
        let mut current_index = 0;
        loop {
            slice[current_index] = (val & 0x7f) as u8;
            val >>= 6;
            if val == 0 || val == -1 {
                break;
            }
            val >>= 1;
            slice[current_index] |= 0x80;
            current_index += 1;
        }

        writer.write_all(&slice[0..=current_index])?;

        Ok(())
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let mut val: i64 = 0;
        let mut shift: u8 = 0;

        for byte in reader.bytes() {
            let byte = byte?;
            val |= ((byte & 0x7f) as i64) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                if shift < 64 && byte & 0x40 != 0 {
                    val |= !0 << shift;
                }
                break;
            }
            if shift >= 64 {
                return Err(invalid_data!("Data out of range for u64"));
            }
        }

        Ok(val)
    }
}

add_serialize_impl! { u64: usize u8 u16 u32 }
add_serialize_impl! { i64: isize i8 i16 i32 }

// Other primitive type serialization

impl Serialize for String {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        let bytes = self.as_bytes();
        writer.serialize(&bytes.len())?;
        writer.write_all(bytes)?;
        Ok(())
    }

    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let len = reader.deserialize()?;
        let mut vec = vec![0; len];
        reader.read_exact(vec.as_mut_slice())?;
        String::from_utf8(vec).map_err(|_| invalid_data!("Invalid data encountered for String of length {len}"))
    }
}

impl Serialize for bool {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_all(&[if *self { 1 } else { 0 }])?;
        Ok(())
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let mut buf = [0];
        reader.read_exact(&mut buf)?;
        let byte = buf[0];
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid_data!("Expected 0 or 1 for bool, got {byte}")),
        }
    }
}

impl Serialize for char {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        u32::serialize(&(*self as u32), writer)
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let raw = u32::deserialize(reader)?;
        if raw <= char::MAX as u32 {
            char::from_u32(raw).ok_or_else(|| invalid_data!("u32 {raw} is not valid char"))
        }else{
            Err(invalid_data!("Integer {} out of bounds for {}", raw, stringify!(char)))
        }
    }
}

// Misc serialization

impl<T: Serialize> Serialize for Option<T> {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        match *self {
            Some(ref val) => {
                writer.serialize(&true)?;
                writer.serialize(val)?;
            },
            None => writer.serialize(&false)?,
        }
        Ok(())
    }

    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let is_some = reader.deserialize()?;
        let val = if is_some {
            Some(reader.deserialize()?)
        }else{
            None
        };
        Ok(val)
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        writer.serialize(&self.len())?;
        for item in self {
            writer.serialize(item)?;
        }
        Ok(())
    }

    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let len = reader.deserialize()?;
        let mut val = Vec::with_capacity(len);
        for _ in 0..len {
            val.push(reader.deserialize()?);
        }
        Ok(val)
    }
}