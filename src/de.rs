use core::{fmt, marker::PhantomData};

use alloc::{boxed::Box, string::ToString, vec};
use serde_core::de::{self, Error as _, IntoDeserializer, Unexpected, Visitor};

use crate::{Error, Owned, Ref, Value};

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Error(msg.to_string())
    }
}

/**
A deserializer that produces values from buffers.

This is the result of calling `into_deserializer` on [`Owned`] or [`Ref`].
*/
pub struct Deserializer<'de>(Value<'de>);

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.0 {
            Value::U8(v) => visitor.visit_u8(v),
            Value::U16(v) => visitor.visit_u16(v),
            Value::U32(v) => visitor.visit_u32(v),
            Value::U64(v) => visitor.visit_u64(v),
            Value::U128(v) => visitor.visit_u128(v),
            Value::I8(v) => visitor.visit_i8(v),
            Value::I16(v) => visitor.visit_i16(v),
            Value::I32(v) => visitor.visit_i32(v),
            Value::I64(v) => visitor.visit_i64(v),
            Value::I128(v) => visitor.visit_i128(v),
            Value::F32(v) => visitor.visit_f32(v),
            Value::F64(v) => visitor.visit_f64(v),
            Value::Bool(v) => visitor.visit_bool(v),
            Value::Char(v) => visitor.visit_char(v),
            Value::Str(v) => visitor.visit_string(v.into()),
            Value::BorrowedStr(v) => visitor.visit_borrowed_str(v),
            Value::Bytes(v) => visitor.visit_byte_buf(v.into_vec()),
            Value::BorrowedBytes(v) => visitor.visit_borrowed_bytes(v),
            Value::None => visitor.visit_none(),
            Value::Some(v) => visitor.visit_some((*v).into_deserializer()),
            Value::Unit => visitor.visit_unit(),
            Value::UnitStruct { name: _ } => visitor.visit_unit(),
            Value::NewtypeStruct { name: _, value } => {
                visitor.visit_newtype_struct(Deserializer(*value))
            }
            Value::Struct { fields, name: _ } => visitor.visit_map(Map::new_str_key(fields)),
            Value::TupleStruct { fields, name: _ } => visitor.visit_seq(Seq::new(fields)),
            Value::Tuple(v) => visitor.visit_seq(Seq::new(v)),
            Value::UnitVariant {
                name: _,
                variant_index,
                variant,
            } => visitor.visit_enum(Enum {
                variant_index,
                variant,
                value: Variant::Value(Value::Unit),
            }),
            Value::NewtypeVariant {
                name: _,
                variant_index,
                variant,
                value,
            } => visitor.visit_enum(Enum {
                variant_index,
                variant,
                value: Variant::Value(*value),
            }),
            Value::TupleVariant {
                name: _,
                variant_index,
                variant,
                fields,
            } => visitor.visit_enum(Enum {
                variant_index,
                variant,
                value: Variant::Tuple(fields),
            }),
            Value::StructVariant {
                name: _,
                variant_index,
                variant,
                fields,
            } => visitor.visit_enum(Enum {
                variant_index,
                variant,
                value: Variant::Struct(fields),
            }),
            Value::Seq(v) => visitor.visit_seq(Seq::new(v)),
            Value::Map(v) => visitor.visit_map(Map::new(v)),
        }
    }

    serde_core::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> IntoDeserializer<'de, Error> for Owned {
    type Deserializer = Deserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        self.0.into_deserializer()
    }
}

impl<'de> IntoDeserializer<'de, Error> for Ref<'de> {
    type Deserializer = Deserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        self.0.into_deserializer()
    }
}

impl<'de> IntoDeserializer<'de, Error> for Value<'de> {
    type Deserializer = Deserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        Deserializer(self)
    }
}

struct Seq<'de>(vec::IntoIter<Value<'de>>);

impl<'de> Seq<'de> {
    fn new(fields: Box<[Value<'de>]>) -> Self {
        Seq(fields.into_vec().into_iter())
    }
}

impl<'de> de::SeqAccess<'de> for Seq<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.0
            .next()
            .map(|field| seed.deserialize(Deserializer(field)))
            .transpose()
    }
}

struct Map<'de, K: IntoDeserializer<'de, E>, E: de::Error> {
    remaining: vec::IntoIter<(K, Value<'de>)>,
    value: Option<Value<'de>>,
    _m: PhantomData<E>,
}

impl<'de> Map<'de, &'de str, de::value::Error> {
    fn new_str_key(fields: Box<[(&'de str, Value<'de>)]>) -> Self {
        Map::new(fields)
    }
}

impl<'de, K: IntoDeserializer<'de, E>, E: de::Error> Map<'de, K, E> {
    fn new(fields: Box<[(K, Value<'de>)]>) -> Self {
        Map {
            remaining: fields.into_vec().into_iter(),
            value: None,
            _m: PhantomData,
        }
    }
}

impl<'de, K: IntoDeserializer<'de, E>, E: de::Error> de::MapAccess<'de> for Map<'de, K, E> {
    type Error = Error;

    fn next_key_seed<D>(&mut self, seed: D) -> Result<Option<D::Value>, Self::Error>
    where
        D: de::DeserializeSeed<'de>,
    {
        if let Some((k, v)) = self.remaining.next() {
            self.value = Some(v);

            Ok(Some(
                seed.deserialize(k.into_deserializer())
                    .map_err(Error::custom)?,
            ))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<D>(&mut self, seed: D) -> Result<D::Value, Self::Error>
    where
        D: de::DeserializeSeed<'de>,
    {
        seed.deserialize(Deserializer(
            self.value
                .take()
                .ok_or_else(|| Error::custom("missing map value"))?,
        ))
    }
}

struct Enum<'de> {
    variant_index: u32,
    variant: &'static str,
    value: Variant<'de>,
}

enum Variant<'de> {
    Value(Value<'de>),
    Tuple(Box<[Value<'de>]>),
    Struct(Box<[(&'static str, Value<'de>)]>),
}

impl<'de> de::EnumAccess<'de> for Enum<'de> {
    type Error = Error;

    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        Ok((
            seed.deserialize(Deserializer(Value::U32(self.variant_index)))?,
            self,
        ))
    }
}

impl<'de> de::VariantAccess<'de> for Enum<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            Variant::Value(Value::Unit) => Ok(()),
            Variant::Value(_) => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
            Variant::Tuple(_) => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
            Variant::Struct(_) => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let value = match self.value {
            Variant::Value(v) => v,
            Variant::Tuple(v) => Value::Tuple(v),
            Variant::Struct(v) => Value::Struct {
                name: self.variant,
                fields: v,
            },
        };

        seed.deserialize(Deserializer(value))
    }

    fn tuple_variant<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Variant::Tuple(v) => visitor.visit_seq(Seq::new(v)),
            Variant::Value(Value::Unit) => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
            Variant::Value(_) => Err(Error::invalid_type(
                Unexpected::NewtypeVariant,
                &"tuple variant",
            )),
            Variant::Struct(_) => Err(Error::invalid_type(
                Unexpected::StructVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Variant::Struct(v) => visitor.visit_map(Map::new_str_key(v)),
            Variant::Value(Value::Unit) => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
            Variant::Value(_) => Err(Error::invalid_type(
                Unexpected::NewtypeVariant,
                &"struct variant",
            )),
            Variant::Tuple(_) => Err(Error::invalid_type(
                Unexpected::TupleVariant,
                &"struct variant",
            )),
        }
    }
}
