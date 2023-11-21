use std::{io::Read, marker::PhantomData};
use serde::de;

use crate::invalid_data;

use super::{Result, Error};

pub struct Deserializer<'de, R: Read> {
    reader: R,
    recurse: usize,
    _phantom_data: PhantomData<&'de R>,
}

impl<'de, R: Read> Deserializer<'de, R> {
    fn recurse<V, F: FnOnce(&mut Self) -> Result<V>>(&mut self, f: F) -> Result<V> {
        if self.recurse == 0 {
            return Err(Error::RecursionLimitExceeded);
        }

        self.recurse -= 1;
        let res = f(self);
        self.recurse += 1;
        res
    }

    #[inline]
    fn get_byte(&mut self) -> Result<u8> {
        let mut buf = [0];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn signed_integer(&mut self) -> Result<i128>{
        let mut val: i128 = 0;
        let mut shift: u8 = 0;

        loop {
            let byte = self.get_byte()?;
            val |= ((byte & 0x7f) as i128) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                if shift < 128 && byte & 0x40 != 0 {
                    val |= !0 << shift;
                }
                break;
            }
            if shift >= 128 {
                return Err(invalid_data!("Data out of range for i128"));
            }
        }

        Ok(val)
    }
    fn unsigned_integer(&mut self) -> Result<u128>{
        let mut val: u128 = 0;
        let mut shift: u8 = 0;

        loop {
            let byte = self.get_byte()?;
            val |= ((byte & 0x7f) as u128) << shift;
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

impl<'de, 'a, R: Read> de::Deserializer<'de> for &'a mut Deserializer<'de, R> {
    type Error = Error;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_i8<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_i16<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_i32<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_i64<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_u8<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_u16<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_u32<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_u64<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_f32<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_f64<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_char<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_str<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_byte_buf<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_unit<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_unit_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_seq<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_tuple<V: de::Visitor<'de>>(self, len: usize, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_tuple_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_map<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_identifier<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_ignored_any<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_i128<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn deserialize_u128<V: de::Visitor<'de>>(self, visitor: V) -> std::result::Result<V::Value, Self::Error> {
        todo!()
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub fn from_reader<T: de::DeserializeOwned> (reader: impl Read) -> Result<T>{
    let d = Deserializer {
        reader,
        recurse: 1024,
    };

    T::deserialize(d)
}