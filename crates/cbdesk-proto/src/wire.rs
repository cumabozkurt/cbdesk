// SPDX-License-Identifier: AGPL-3.0-or-later

//! A small protobuf wire codec.
//!
//! The codec keeps field numbers it does not know and writes them back out.
//! That is what lets a newer peer add a field without the older peer dropping
//! it on a decode/encode round trip. Known fields are written in field-number
//! order; unknown fields keep the order they arrived in.

use crate::ProtoError;

const WT_VARINT: u8 = 0;
const WT_I64: u8 = 1;
const WT_LEN: u8 = 2;
const WT_I32: u8 = 5;

/// One decoded protobuf value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldValue {
    /// An unsigned varint.
    Varint(u64),
    /// A 64-bit fixed value.
    Fixed64(u64),
    /// Length-delimited bytes (string, embedded message, or packed repeated).
    Len(Vec<u8>),
    /// A 32-bit fixed value.
    Fixed32(u32),
}

/// Fields this version of the codec does not recognize.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnknownFields {
    fields: Vec<(u32, FieldValue)>,
}

impl UnknownFields {
    /// No unknown fields.
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// True when nothing unknown was retained.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Field number and value, in arrival order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &FieldValue)> {
        self.fields.iter().map(|(n, v)| (*n, v))
    }

    #[cfg(test)]
    pub(crate) fn push(&mut self, number: u32, value: FieldValue) {
        self.fields.push((number, value));
    }

    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        for (number, value) in &self.fields {
            encode_value(out, *number, value);
        }
    }
}

/// Parsed fields waiting to be consumed by a message decoder.
#[derive(Debug)]
pub struct Fields {
    items: Vec<(u32, FieldValue)>,
}

impl Fields {
    /// Parse a protobuf message body.
    pub fn parse(bytes: &[u8]) -> Result<Self, ProtoError> {
        let mut cursor = Cursor::new(bytes);
        let mut items = Vec::new();
        while !cursor.is_empty() {
            let key = cursor.read_varint()?;
            if key > u64::from(u32::MAX) {
                return Err(ProtoError::BadFieldNumber);
            }
            let number = (key >> 3) as u32;
            let wire_type = (key & 7) as u8;
            if number == 0 {
                return Err(ProtoError::BadFieldNumber);
            }
            let value = match wire_type {
                WT_VARINT => FieldValue::Varint(cursor.read_varint()?),
                WT_I64 => FieldValue::Fixed64(cursor.read_fixed64()?),
                WT_LEN => FieldValue::Len(cursor.read_bytes()?.to_vec()),
                WT_I32 => FieldValue::Fixed32(cursor.read_fixed32()?),
                other => return Err(ProtoError::BadWireType(other)),
            };
            items.push((number, value));
        }
        Ok(Self { items })
    }

    pub(crate) fn take_u32(&mut self, number: u32) -> Result<u32, ProtoError> {
        let values = self.take_varints(number)?;
        match values.last() {
            None => Ok(0),
            Some(&v) => u32::try_from(v).map_err(|_| ProtoError::Overflow),
        }
    }

    pub(crate) fn take_u64(&mut self, number: u32) -> Result<u64, ProtoError> {
        let values = self.take_varints(number)?;
        Ok(values.last().copied().unwrap_or(0))
    }

    pub(crate) fn take_i32(&mut self, number: u32) -> Result<i32, ProtoError> {
        let values = self.take_varints(number)?;
        match values.last() {
            None => Ok(0),
            Some(&v) => varint_to_i32(v),
        }
    }

    pub(crate) fn take_bool(&mut self, number: u32) -> Result<bool, ProtoError> {
        let values = self.take_varints(number)?;
        Ok(values.last().copied().unwrap_or(0) != 0)
    }

    pub(crate) fn take_bytes(&mut self, number: u32) -> Result<Vec<u8>, ProtoError> {
        let taken = self.take_matching(number);
        let mut last = Vec::new();
        let mut seen = false;
        for value in taken {
            match value {
                FieldValue::Len(bytes) => {
                    last = bytes;
                    seen = true;
                }
                _ => return Err(ProtoError::WrongWireType(number)),
            }
        }
        if seen {
            Ok(last)
        } else {
            Ok(Vec::new())
        }
    }

    pub(crate) fn take_string(&mut self, number: u32) -> Result<String, ProtoError> {
        let bytes = self.take_bytes(number)?;
        String::from_utf8(bytes).map_err(|_| ProtoError::BadUtf8)
    }

    pub(crate) fn take_f32(&mut self, number: u32) -> Result<f32, ProtoError> {
        let taken = self.take_matching(number);
        let mut last = 0u32;
        for value in taken {
            match value {
                FieldValue::Fixed32(bits) => last = bits,
                _ => return Err(ProtoError::WrongWireType(number)),
            }
        }
        Ok(f32::from_bits(last))
    }

    /// Repeated enums: packed (proto3 default) or unpacked.
    pub(crate) fn take_packed_i32(&mut self, number: u32) -> Result<Vec<i32>, ProtoError> {
        let taken = self.take_matching(number);
        let mut out = Vec::new();
        for value in taken {
            match value {
                FieldValue::Varint(v) => out.push(varint_to_i32(v)?),
                FieldValue::Len(bytes) => {
                    let mut cursor = Cursor::new(&bytes);
                    while !cursor.is_empty() {
                        out.push(varint_to_i32(cursor.read_varint()?)?);
                    }
                }
                _ => return Err(ProtoError::WrongWireType(number)),
            }
        }
        Ok(out)
    }

    pub(crate) fn take_messages<T: DecodedMessage>(
        &mut self,
        number: u32,
    ) -> Result<Vec<T>, ProtoError> {
        let taken = self.take_matching(number);
        let mut out = Vec::new();
        for value in taken {
            match value {
                FieldValue::Len(bytes) => out.push(T::decode_limited(&bytes)?),
                _ => return Err(ProtoError::WrongWireType(number)),
            }
        }
        Ok(out)
    }

    pub(crate) fn finish_unknown(&mut self) -> UnknownFields {
        UnknownFields {
            fields: std::mem::take(&mut self.items),
        }
    }

    pub(crate) fn take_repeated_bytes(&mut self, number: u32) -> Result<Vec<Vec<u8>>, ProtoError> {
        let taken = self.take_matching(number);
        let mut out = Vec::with_capacity(taken.len());
        for value in taken {
            match value {
                FieldValue::Len(bytes) => out.push(bytes),
                _ => return Err(ProtoError::WrongWireType(number)),
            }
        }
        Ok(out)
    }

    fn take_varints(&mut self, number: u32) -> Result<Vec<u64>, ProtoError> {
        let taken = self.take_matching(number);
        let mut out = Vec::new();
        for value in taken {
            match value {
                FieldValue::Varint(v) => out.push(v),
                _ => return Err(ProtoError::WrongWireType(number)),
            }
        }
        Ok(out)
    }

    fn take_matching(&mut self, number: u32) -> Vec<FieldValue> {
        let mut kept = Vec::new();
        let mut taken = Vec::new();
        for (n, value) in self.items.drain(..) {
            if n == number {
                taken.push(value);
            } else {
                kept.push((n, value));
            }
        }
        self.items = kept;
        taken
    }
}

pub(crate) trait DecodedMessage: Sized {
    fn decode_limited(bytes: &[u8]) -> Result<Self, ProtoError>;
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn read_varint(&mut self) -> Result<u64, ProtoError> {
        let mut value = 0u64;
        let mut shift = 0;
        loop {
            if shift >= 64 {
                return Err(ProtoError::BadVarint);
            }
            if self.pos >= self.buf.len() {
                return Err(ProtoError::Truncated);
            }
            let byte = self.buf[self.pos];
            self.pos += 1;
            let bits = u64::from(byte & 0x7f);
            if shift == 63 && bits > 1 {
                return Err(ProtoError::BadVarint);
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
    }

    fn read_fixed32(&mut self) -> Result<u32, ProtoError> {
        let bytes = self.read_array::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_fixed64(&mut self) -> Result<u64, ProtoError> {
        let bytes = self.read_array::<8>()?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_bytes(&mut self) -> Result<&'a [u8], ProtoError> {
        let len = self.read_varint()?;
        let len = usize::try_from(len).map_err(|_| ProtoError::Overflow)?;
        let end = self.pos.checked_add(len).ok_or(ProtoError::Overflow)?;
        if end > self.buf.len() {
            return Err(ProtoError::Truncated);
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ProtoError> {
        let end = self.pos.checked_add(N).ok_or(ProtoError::Truncated)?;
        if end > self.buf.len() {
            return Err(ProtoError::Truncated);
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.pos..end]);
        self.pos = end;
        Ok(out)
    }
}

fn varint_to_i32(value: u64) -> Result<i32, ProtoError> {
    i32::try_from(value as i64).map_err(|_| ProtoError::Overflow)
}

fn encode_value(out: &mut Vec<u8>, number: u32, value: &FieldValue) {
    match value {
        FieldValue::Varint(v) => {
            write_key(out, number, WT_VARINT);
            write_varint(out, *v);
        }
        FieldValue::Fixed64(v) => {
            write_key(out, number, WT_I64);
            out.extend_from_slice(&v.to_le_bytes());
        }
        FieldValue::Len(bytes) => {
            write_bytes(out, number, bytes);
        }
        FieldValue::Fixed32(v) => {
            write_key(out, number, WT_I32);
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

pub(crate) fn write_key(out: &mut Vec<u8>, field: u32, wire_type: u8) {
    write_varint(out, (u64::from(field) << 3) | u64::from(wire_type));
}

pub(crate) fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
            out.push(byte);
        } else {
            out.push(byte);
            break;
        }
    }
}

pub(crate) fn write_u32(out: &mut Vec<u8>, field: u32, value: u32) {
    if value == 0 {
        return;
    }
    write_key(out, field, WT_VARINT);
    write_varint(out, u64::from(value));
}

pub(crate) fn write_u64(out: &mut Vec<u8>, field: u32, value: u64) {
    if value == 0 {
        return;
    }
    write_key(out, field, WT_VARINT);
    write_varint(out, value);
}

pub(crate) fn write_i32(out: &mut Vec<u8>, field: u32, value: i32) {
    if value == 0 {
        return;
    }
    write_key(out, field, WT_VARINT);
    write_varint(out, value as i64 as u64);
}

pub(crate) fn write_bool(out: &mut Vec<u8>, field: u32, value: bool) {
    if !value {
        return;
    }
    write_key(out, field, WT_VARINT);
    write_varint(out, 1);
}

pub(crate) fn write_bytes(out: &mut Vec<u8>, field: u32, value: &[u8]) {
    write_key(out, field, WT_LEN);
    write_varint(out, value.len() as u64);
    out.extend_from_slice(value);
}

pub(crate) fn write_string(out: &mut Vec<u8>, field: u32, value: &str) {
    if value.is_empty() {
        return;
    }
    write_bytes(out, field, value.as_bytes());
}

pub(crate) fn write_f32(out: &mut Vec<u8>, field: u32, value: f32) -> Result<(), ProtoError> {
    if !value.is_finite() {
        return Err(ProtoError::NonFinite);
    }
    if value.to_bits() == 0 {
        return Ok(());
    }
    write_key(out, field, WT_I32);
    out.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

pub(crate) fn write_packed_i32(out: &mut Vec<u8>, field: u32, values: &[i32]) {
    if values.is_empty() {
        return;
    }
    let mut packed = Vec::new();
    for value in values {
        write_varint(&mut packed, *value as i64 as u64);
    }
    write_bytes(out, field, &packed);
}

pub(crate) fn write_message(out: &mut Vec<u8>, field: u32, bytes: &[u8]) {
    write_bytes(out, field, bytes);
}
