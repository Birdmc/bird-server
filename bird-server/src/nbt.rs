use std::borrow::Cow;
use std::collections::HashMap;
use bird_protocol::{anyhow, ProtocolCursor, ProtocolError, ProtocolReadable, ProtocolResult, ProtocolWritable, ProtocolWriter};

#[derive(Clone, Debug, PartialEq)]
pub enum NbtElement<'a> {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(&'a [u8]),
    String(Cow<'a, str>),
    List(Vec<NbtElement<'a>>),
    Compound(HashMap<Cow<'a, str>, NbtElement<'a>>),
    IntArray(&'a [u8]), // in little endian
    LongArray(&'a [u8]), // in little endian
}

pub fn read_compound_enter<'a, C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<()> {
    let ty = i8::read(cursor)?;
    debug_assert!(ty == 10);
    let _ = read_nbt_string(cursor)?;
    Ok(())
}

pub fn read_named_nbt_tag<'a, C: ProtocolCursor<'a>>(name: &str, cursor: &mut C) -> ProtocolResult<Option<NbtElement<'a>>> {
    let mut result = None;
    loop {
        let id = i8::read(cursor)?;
        if id == 0 { break; }
        let tag_name = read_nbt_string(cursor)?;
        let tag_value = read_nbt_tag(id, cursor)?;
        if result.is_none() && tag_name == name { result = Some(tag_value); }
    }
    Ok(result)
}

pub fn read_nbt_string<'a, C: ProtocolCursor<'a>>(cursor: &mut C) -> ProtocolResult<Cow<'a, str>> {
    let length = u16::read(cursor)?;
    cesu8::from_java_cesu8(cursor.take_bytes(length as usize)?)
        .map_err(|err| ProtocolError::Any(err.into()))
}

pub fn read_nbt_tag<'a, C: ProtocolCursor<'a>>(id: i8, cursor: &mut C) -> ProtocolResult<NbtElement<'a>> {
    Ok(match id {
        0 => NbtElement::End,
        1 => NbtElement::Byte(i8::read(cursor)?),
        2 => NbtElement::Short(i16::read(cursor)?),
        3 => NbtElement::Int(i32::read(cursor)?),
        4 => NbtElement::Long(i64::read(cursor)?),
        5 => NbtElement::Float(f32::read(cursor)?),
        6 => NbtElement::Double(f64::read(cursor)?),
        7 => {
            let length = i32::read(cursor)?;
            NbtElement::ByteArray(cursor.take_bytes(length as usize)?)
        },
        8 => NbtElement::String(read_nbt_string(cursor)?),
        9 => NbtElement::List({
            let id = i8::read(cursor)?;
            let length = i32::read(cursor)?;
            match length <= 0 {
                true => Vec::new(),
                false => match id == 0 {
                    true => Err(ProtocolError::Any(anyhow::Error::msg("NBTEnd when NbtList is not empty")))?,
                    false => {
                        let mut result = Vec::new();
                        for _ in 0..length {
                            result.push(read_nbt_tag(id, cursor)?);
                        }
                        result
                    }
                }
            }
        }),
        10 => NbtElement::Compound({
            let mut result = HashMap::new();
            loop {
                let tag = i8::read(cursor)?;
                if tag == 0 { break; }
                let name = read_nbt_string(cursor)?;
                let element = read_nbt_tag(tag, cursor)?;
                result.insert(name, element);
            }
            result
        }),
        11 => {
            let length = i32::read(cursor)?;
            NbtElement::IntArray(cursor.take_bytes(length as usize * 4)?)
        },
        12 => {
            let length = i32::read(cursor)?;
            NbtElement::LongArray(cursor.take_bytes(length as usize * 8)?)
        },
        _ => Err(ProtocolError::Any(anyhow::Error::msg("Only tags from 0 to 12 are supported")))?
    })
}

pub fn write_compound_enter<W: ProtocolWriter>(writer: &mut W) -> anyhow::Result<()> {
    10i8.write(writer)?;
    write_nbt_string("_", writer)
}

pub fn write_nbt_string<W: ProtocolWriter>(str: &str, writer: &mut W) -> anyhow::Result<()> {
    match str.len() > u16::MAX as _ {
        true => Err(anyhow::Error::msg("Too big string")),
        false => {
            (str.len() as u16).write(writer)?;
            Ok(match cesu8::to_java_cesu8(str) {
                Cow::Owned(ref owned) => writer.write_bytes(owned),
                Cow::Borrowed(borrowed) => writer.write_bytes(borrowed),
            })
        }
    }
}

pub fn nbt_key(element: &NbtElement) -> i8 {
    match element {
        NbtElement::End => 0,
        NbtElement::Byte(_) => 1,
        NbtElement::Short(_) => 2,
        NbtElement::Int(_) => 3,
        NbtElement::Long(_) => 4,
        NbtElement::Float(_) => 5,
        NbtElement::Double(_) => 6,
        NbtElement::ByteArray(_) => 7,
        NbtElement::String(_) => 8,
        NbtElement::List(_) => 9,
        NbtElement::Compound(_) => 10,
        NbtElement::IntArray(_) => 11,
        NbtElement::LongArray(_) => 12,
    }
}

pub fn write_nbt_element<W: ProtocolWriter>(element: &NbtElement, writer: &mut W) -> anyhow::Result<()> {
    Ok(match element {
        NbtElement::End => {}
        NbtElement::Byte(n) => n.write(writer)?,
        NbtElement::Short(n) => n.write(writer)?,
        NbtElement::Int(n) => n.write(writer)?,
        NbtElement::Long(n) => n.write(writer)?,
        NbtElement::Float(n) => n.write(writer)?,
        NbtElement::Double(n) => n.write(writer)?,
        NbtElement::ByteArray(array) => {
            (array.len() as i32).write(writer)?;
            writer.write_bytes(array)
        }
        NbtElement::String(str) => write_nbt_string(str, writer)?,
        NbtElement::List(_) => unimplemented!(),
        NbtElement::Compound(_) => unimplemented!(),
        NbtElement::IntArray(_) => unimplemented!(),
        NbtElement::LongArray(_) => unimplemented!(),
    })
}