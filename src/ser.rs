use core::{cmp, fmt, marker::PhantomData};

use alloc::{borrow::ToOwned, boxed::Box, string::ToString, vec::Vec};
use serde::{
    ser::{
        self, Error as _, SerializeMap as _, SerializeSeq as _, SerializeStruct as _,
        SerializeStructVariant as _, SerializeTuple as _, SerializeTupleStruct as _,
        SerializeTupleVariant as _,
    },
    Serialize,
};

use crate::{Error, Owned, Ref, Value};

impl<'a> Serialize for Ref<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl Serialize for Owned {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'a> Serialize for Value<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Value::Unit => serializer.serialize_unit(),
            Value::U8(v) => serializer.serialize_u8(v),
            Value::U16(v) => serializer.serialize_u16(v),
            Value::U32(v) => serializer.serialize_u32(v),
            Value::U64(v) => serializer.serialize_u64(v),
            Value::U128(v) => serializer.serialize_u128(v),
            Value::I8(v) => serializer.serialize_i8(v),
            Value::I16(v) => serializer.serialize_i16(v),
            Value::I32(v) => serializer.serialize_i32(v),
            Value::I64(v) => serializer.serialize_i64(v),
            Value::I128(v) => serializer.serialize_i128(v),
            Value::F32(v) => serializer.serialize_f32(v),
            Value::F64(v) => serializer.serialize_f64(v),
            Value::Bool(v) => serializer.serialize_bool(v),
            Value::Char(v) => serializer.serialize_char(v),
            Value::Str(ref v) => serializer.serialize_str(&v),
            Value::BorrowedStr(v) => serializer.serialize_str(v),
            Value::Bytes(ref v) => serializer.serialize_bytes(v),
            Value::BorrowedBytes(v) => serializer.serialize_bytes(v),
            Value::None => serializer.serialize_none(),
            Value::Some(ref v) => serializer.serialize_some(v),
            Value::UnitStruct { name } => serializer.serialize_unit_struct(name),
            Value::NewtypeStruct { name, ref value } => {
                serializer.serialize_newtype_struct(name, value)
            }
            Value::Struct { name, ref fields } => {
                let mut serializer = serializer.serialize_struct(name, fields.len())?;

                for (name, field) in &**fields {
                    serializer.serialize_field(name, field)?;
                }

                serializer.end()
            }
            Value::TupleStruct { name, ref fields } => {
                let mut serializer = serializer.serialize_tuple_struct(name, fields.len())?;

                for field in &**fields {
                    serializer.serialize_field(field)?;
                }

                serializer.end()
            }
            Value::Tuple(ref v) => {
                let mut serializer = serializer.serialize_tuple(v.len())?;

                for field in &**v {
                    serializer.serialize_element(field)?;
                }

                serializer.end()
            }
            Value::UnitVariant {
                name,
                variant_index,
                variant,
            } => serializer.serialize_unit_variant(name, variant_index, variant),
            Value::NewtypeVariant {
                name,
                variant_index,
                variant,
                ref value,
            } => serializer.serialize_newtype_variant(name, variant_index, variant, value),
            Value::TupleVariant {
                name,
                variant_index,
                variant,
                ref fields,
            } => {
                let mut serializer = serializer.serialize_tuple_variant(
                    name,
                    variant_index,
                    variant,
                    fields.len(),
                )?;

                for field in &**fields {
                    serializer.serialize_field(field)?;
                }

                serializer.end()
            }
            Value::StructVariant {
                name,
                variant_index,
                variant,
                ref fields,
            } => {
                let mut serializer = serializer.serialize_struct_variant(
                    name,
                    variant_index,
                    variant,
                    fields.len(),
                )?;

                for (name, field) in &**fields {
                    serializer.serialize_field(name, field)?;
                }

                serializer.end()
            }
            Value::Seq(ref v) => {
                let mut serializer = serializer.serialize_seq(Some(v.len()))?;

                for field in &**v {
                    serializer.serialize_element(field)?;
                }

                serializer.end()
            }
            Value::Map(ref v) => {
                let mut serializer = serializer.serialize_map(Some(v.len()))?;

                for (key, value) in &**v {
                    serializer.serialize_entry(key, value)?;
                }

                serializer.end()
            }
        }
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Error(msg.to_string())
    }
}

/**
A serializer that produces [`Owned`] buffers from an arbitrary [`serde::Serialize`].
*/
pub struct Serializer(PhantomData<()>);

impl Serializer {
    /**
    Create a new serializer for an [`Owned`] buffer.
    */
    pub fn new() -> Self {
        Serializer(PhantomData)
    }
}

pub struct SerializeSeq {
    fields: Vec<Value<'static>>,
}

pub struct SerializeTuple {
    fields: Vec<Value<'static>>,
}

pub struct SerializeTupleStruct {
    name: &'static str,
    fields: Vec<Value<'static>>,
}

pub struct SerializeTupleVariant {
    name: &'static str,
    variant_index: u32,
    variant: &'static str,
    fields: Vec<Value<'static>>,
}

pub struct SerializeMap {
    key: Option<Value<'static>>,
    fields: Vec<(Value<'static>, Value<'static>)>,
}

pub struct SerializeStruct {
    name: &'static str,
    fields: Vec<(&'static str, Value<'static>)>,
}

/**
A serializer that produces [`Owned`] buffers from struct variants.
*/
pub struct SerializeStructVariant {
    name: &'static str,
    variant_index: u32,
    variant: &'static str,
    fields: Vec<(&'static str, Value<'static>)>,
}

impl serde::Serializer for Serializer {
    type Ok = Owned;
    type Error = Error;
    type SerializeSeq = SerializeSeq;
    type SerializeTuple = SerializeTuple;
    type SerializeTupleStruct = SerializeTupleStruct;
    type SerializeTupleVariant = SerializeTupleVariant;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeStruct;
    type SerializeStructVariant = SerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Bool(v)))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::I8(v)))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::I16(v)))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::I32(v)))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::I64(v)))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::I128(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::U8(v)))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::U16(v)))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::U32(v)))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::U64(v)))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::U128(v)))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::F32(v)))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::F64(v)))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Char(v)))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Str(v.to_owned())))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Bytes(v.to_owned())))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::None))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Ok(Owned(Value::Some(Box::new(value.serialize(Serializer::new())?.0))))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Unit))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::UnitStruct { name }))
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::UnitVariant {
            name,
            variant_index,
            variant,
        }))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Ok(Owned(Value::NewtypeStruct {
            name,
            value: Box::new(value.serialize(Serializer::new())?.0),
        }))
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Ok(Owned(Value::NewtypeVariant {
            name,
            variant_index,
            variant,
            value: Box::new(value.serialize(Serializer::new())?.0),
        }))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeSeq {
            fields: Vec::with_capacity(cmp::min(len.unwrap_or(0), 32)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerializeTuple {
            fields: Vec::with_capacity(cmp::min(len, 32)),
        })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SerializeTupleStruct {
            name,
            fields: Vec::with_capacity(cmp::min(len, 32)),
        })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant {
            name,
            variant_index,
            variant,
            fields: Vec::with_capacity(cmp::min(len, 32)),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap {
            key: None,
            fields: Vec::with_capacity(cmp::min(len.unwrap_or(0), 32)),
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeStruct {
            name,
            fields: Vec::with_capacity(cmp::min(len, 32)),
        })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant {
            name,
            variant_index,
            variant,
            fields: Vec::with_capacity(cmp::min(len, 32)),
        })
    }
}

impl ser::SerializeSeq for SerializeSeq {
    type Ok = Owned;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.fields.push(value.serialize(Serializer::new())?.0);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Seq(self.fields.into_boxed_slice())))
    }
}

impl ser::SerializeMap for SerializeMap {
    type Ok = Owned;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if self.key.is_some() {
            return Err(Error::custom("missing map value"));
        }

        self.key = Some(key.serialize(Serializer::new())?.0);

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::custom("missing map key"))?;
        let value = value.serialize(Serializer::new())?.0;

        self.fields.push((key, value));

        Ok(())
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        if self.key.is_some() {
            return Err(Error::custom("missing map value"));
        }

        let key = key.serialize(Serializer::new())?.0;
        let value = value.serialize(Serializer::new())?.0;

        self.fields.push((key, value));

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.key.is_some() {
            return Err(Error::custom("missing map value"));
        }

        Ok(Owned(Value::Map(self.fields.into_boxed_slice())))
    }
}

impl ser::SerializeStruct for SerializeStruct {
    type Ok = Owned;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.fields.push((key, value.serialize(Serializer::new())?.0));

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Struct {
            name: self.name,
            fields: self.fields.into_boxed_slice(),
        }))
    }
}

impl ser::SerializeStructVariant for SerializeStructVariant {
    type Ok = Owned;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.fields.push((key, value.serialize(Serializer::new())?.0));

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::StructVariant {
            name: self.name,
            variant_index: self.variant_index,
            variant: self.variant,
            fields: self.fields.into_boxed_slice(),
        }))
    }
}

impl ser::SerializeTuple for SerializeTuple {
    type Ok = Owned;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.fields.push(value.serialize(Serializer::new())?.0);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::Tuple(self.fields.into_boxed_slice())))
    }
}

impl ser::SerializeTupleStruct for SerializeTupleStruct {
    type Ok = Owned;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.fields.push(value.serialize(Serializer::new())?.0);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::TupleStruct {
            name: self.name,
            fields: self.fields.into_boxed_slice(),
        }))
    }
}

impl ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Owned;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.fields.push(value.serialize(Serializer::new())?.0);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Owned(Value::TupleVariant {
            name: self.name,
            variant_index: self.variant_index,
            variant: self.variant,
            fields: self.fields.into_boxed_slice(),
        }))
    }
}
