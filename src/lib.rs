/*!
Generic buffering for `serde`.

This library provides the [`Owned`] and [`Ref`] types as format-independent buffers for `serde`.
Buffers are guaranteed to serialize in exactly the same way as their original source.

# Getting an owned buffer

Any type that implements [`serde::Serialize`] can be buffered into an [`Owned`] buffer:

```
# use std::cell::RefCell;
use serde_buf::Owned;

# fn main() -> Result<(), serde_buf::Error> {
// Imagine we have some owned resource...
thread_local! {
    static SOME_THREAD_STATIC: RefCell<Option<Owned>> = RefCell::new(None);
}

// ...and some short-lived data
let short_lived: &'_ str = "A string";

// We can create an owned buffer from it...
let buffer = Owned::buffer(short_lived)?;

// ...and stash it in our owned resource
SOME_THREAD_STATIC.with(|ts| *ts.borrow_mut() = Some(buffer));
# Ok(())
# }
```

# Getting a borrowed buffer

Borrowed [`Ref`] buffers may have internally borrowed strings, which makes them incompatible
with [`serde::Serialize`]. You can construct a [`Ref`] manually from any underlying source:

```
use serde_buf::Ref;

// Imagine we have some borrowed datatype
struct MyData<'a> {
    id: u64,
    content: &'a str,
}

// We can buffer it into a partially owned buffer manually
fn buffer_my_data<'a>(data: &'_ MyData<'a>) -> Ref<'a> {
    Ref::record_struct("MyData", [
        ("id", Ref::u64(data.id)),
        ("content", Ref::str(data.content)),
    ])
}
```

# Serializing a buffer

Once you've got an [`Owned`] or [`Ref`] buffer, you can then later use their own [`serde::Serialize`]
implementations to encode in some format:

```
# use serde::Serialize;
# use serde_derive::Serialize;
# use serde_buf::Owned;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[derive(Serialize)]
struct MyData<'a> {
    id: u64,
    content: &'a str,
}

let data = MyData {
    id: 42,
    content: "Some content",
};

let buffer = Owned::buffer(&data)?;

let data_json = serde_json::to_string(&data)?;
let buffer_json = serde_json::to_string(&buffer)?;

assert_eq!(data_json, buffer_json);
# Ok(())
# }
```

# Deserializing from a buffer

Values can also be deserialized directly from an [`Owned`] or [`Ref`] buffer through their [`serde::de::IntoDeserializer`]
implementations:

```
# fn main() -> Result<(), serde_buf::Error> {
# fn buffer_my_data<'a>(data: &'_ MyData<'a>) -> Ref<'a> {
#     Ref::record_struct("MyData", [
#         ("id", Ref::u64(data.id)),
#         ("content", Ref::str(data.content)),
#     ])
# }
# use serde::{Deserialize, de::IntoDeserializer};
# use serde_derive::Deserialize;
# use serde_buf::Ref;
#[derive(Deserialize, Debug, PartialEq)]
struct MyData<'a> {
    id: u64,
    content: &'a str,
}

let data = MyData {
    id: 42,
    content: "Some content",
};

let buffer: Ref = buffer_my_data(&data);

let deserialized = MyData::deserialize(buffer.into_deserializer())?;

assert_eq!(data, deserialized);
# Ok(())
# }
```

# Deserializing directly to a buffer

The [`Ref`] and [`Owned`] types don't implement [`serde::Deserialize`] and can't be deserialized directly.
This is because `serde` relies on hints from the target type to know how to interpret enum variants.
If a type implements both [`serde::Serialize`] and [`serde::Deserialize`] then you can first deserialize to
that concrete type, and then buffer it:

```
# use serde::Deserialize;
# use serde_buf::Owned;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# fn data_json() -> String { serde_json::to_string(&MyData::Full { id: 42, content: "Some content" }).unwrap() }
# use serde_derive::{Serialize, Deserialize};
#[derive(Serialize, Deserialize)]
enum MyData<'a> {
    Short(&'a str),
    Full { id: u64, content: &'a str },
}

let json = data_json();

let buffer = Owned::buffer(&serde_json::from_str::<MyData>(&json)?)?;
# Ok(())
# }
```
*/

#![deny(missing_docs)]
#![no_std]

extern crate alloc;

use core::{borrow::Borrow, fmt};

use alloc::{boxed::Box, string::String, vec::Vec};
use serde::Serialize;

mod de;
mod ser;

pub use self::{de::Deserializer, ser::Serializer};

/**
An error encountered while buffering a value.
*/
#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error buffering a value")
    }
}

impl serde::ser::StdError for Error {}

/**
A fully owned value.
*/
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Owned(Value<'static>);

impl From<Ref<'static>> for Owned {
    fn from(value: Ref<'static>) -> Self {
        Owned(value.0)
    }
}

impl Owned {
    /**
    Buffer `v` into an owned buffer.

    The resulting buffer is guaranteed to serialize to the same value as `v`.
    */
    pub fn buffer(v: impl Serialize) -> Result<Self, Error> {
        v.serialize(Serializer::new())
    }
}

/**
A partly owned value.

This buffer allows strings to be borrowed internally.
*/
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Ref<'a>(Value<'a>);

impl From<Owned> for Ref<'static> {
    fn from(value: Owned) -> Self {
        Ref(value.0)
    }
}

impl Ref<'static> {
    /**
    Buffer `v` into an owned buffer.

    The resulting buffer is guaranteed to serialize to the same value as `v`.
    */
    pub fn buffer(v: impl Serialize) -> Result<Self, Error> {
        Ok(v.serialize(Serializer::new())?.into())
    }
}

impl<'a> Ref<'a> {
    /**
    Create a buffer for a `()` value.
    */
    pub fn unit() -> Self {
        Ref(Value::Unit)
    }

    /**
    Create a buffer for a boolean value.
    */
    pub fn bool(v: bool) -> Self {
        Ref(Value::Bool(v))
    }

    /**
    Create a buffer for an unsigned integer value.
    */
    pub fn u8(v: u8) -> Self {
        Ref(Value::U8(v))
    }

    /**
    Create a buffer for an unsigned integer value.
    */
    pub fn u16(v: u16) -> Self {
        Ref(Value::U16(v))
    }

    /**
    Create a buffer for an unsigned integer value.
    */
    pub fn u32(v: u32) -> Self {
        Ref(Value::U32(v))
    }

    /**
    Create a buffer for an unsigned integer value.
    */
    pub fn u64(v: u64) -> Self {
        Ref(Value::U64(v))
    }

    /**
    Create a buffer for an unsigned integer value.
    */
    pub fn u128(v: u128) -> Self {
        Ref(Value::U128(v))
    }

    /**
    Create a buffer for a signed integer value.
    */
    pub fn i8(v: i8) -> Self {
        Ref(Value::I8(v))
    }

    /**
    Create a buffer for a signed integer value.
    */
    pub fn i16(v: i16) -> Self {
        Ref(Value::I16(v))
    }

    /**
    Create a buffer for a signed integer value.
    */
    pub fn i32(v: i32) -> Self {
        Ref(Value::I32(v))
    }

    /**
    Create a buffer for a signed integer value.
    */
    pub fn i64(v: i64) -> Self {
        Ref(Value::I64(v))
    }

    /**
    Create a buffer for a signed integer value.
    */
    pub fn i128(v: i128) -> Self {
        Ref(Value::I128(v))
    }

    /**
    Create a buffer for a binary floating point value.
    */
    pub fn f32(v: f32) -> Self {
        Ref(Value::F32(v))
    }

    /**
    Create a buffer for a binary floating point value.
    */
    pub fn f64(v: f64) -> Self {
        Ref(Value::F64(v))
    }

    /**
    Create a buffer for a single character value.
    */
    pub fn char(v: char) -> Self {
        Ref(Value::Char(v))
    }

    /**
    Create a buffer for an owned string value.
    */
    pub fn owned_str(v: impl Into<String>) -> Self {
        Ref(Value::Str(v.into().into_boxed_str()))
    }

    /**
    Create a buffer for a borrowed string value.
    */
    pub fn str(v: &'a (impl Borrow<str> + ?Sized)) -> Self {
        Ref(Value::BorrowedStr(v.borrow()))
    }

    /**
    Create a buffer for an owned byte-string value.
    */
    pub fn owned_bytes(v: impl Into<Vec<u8>>) -> Self {
        Ref(Value::Bytes(v.into().into_boxed_slice()))
    }

    /**
    Create a buffer for a borrowed byte-string value.
    */
    pub fn bytes(v: &'a (impl Borrow<[u8]> + ?Sized)) -> Self {
        Ref(Value::BorrowedBytes(v.borrow()))
    }

    /**
    Create a buffer for an `Option::None` value.
    */
    pub fn none() -> Self {
        Ref(Value::None)
    }

    /**
    Create a buffer for an `Option::Some` value.
    */
    pub fn some(v: impl Into<Ref<'a>>) -> Self {
        Ref(Value::Some(Box::new(v.into().0)))
    }

    /**
    Create a buffer for a unit struct, like `struct A`.
    */
    pub fn unit_struct(name: &'static str) -> Self {
        Ref(Value::UnitStruct { name })
    }

    /**
    Create a buffer for a newtype struct, like `struct A(T)`.
    */
    pub fn newtype_struct(name: &'static str, value: impl Into<Ref<'a>>) -> Self {
        Ref(Value::NewtypeStruct {
            name,
            value: Box::new(value.into().0),
        })
    }

    /**
    Create a buffer for a struct with named fields, like `struct A { a: T, b: U }`.
    */
    pub fn record_struct(
        name: &'static str,
        fields: impl IntoIterator<Item = (&'static str, Ref<'a>)>,
    ) -> Self {
        Ref(Value::Struct {
            name,
            fields: fields
                .into_iter()
                .map(|(k, v)| (k, v.0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
    }

    /**
    Create a buffer for a struct with unnamed fields, like `struct A(T, U)`.
    */
    pub fn tuple_struct(name: &'static str, fields: impl IntoIterator<Item = Ref<'a>>) -> Self {
        Ref(Value::TupleStruct {
            name,
            fields: fields
                .into_iter()
                .map(|v| v.0)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
    }

    /**
    Create a buffer for a tuple, like `(T, U)`.
    */
    pub fn tuple(fields: impl IntoIterator<Item = Ref<'a>>) -> Self {
        Ref(Value::Tuple(
            fields
                .into_iter()
                .map(|v| v.0)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ))
    }

    /**
    Create a buffer for a unit enum variant, like `A::B`.
    */
    pub fn unit_variant(name: &'static str, variant_index: u32, variant: &'static str) -> Self {
        Ref(Value::UnitVariant {
            name,
            variant_index,
            variant,
        })
    }

    /**
    Create a buffer for a newtype enum variant, like `A::B(T)`.
    */
    pub fn newtype_variant(
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: impl Into<Ref<'a>>,
    ) -> Self {
        Ref(Value::NewtypeVariant {
            name,
            variant_index,
            variant,
            value: Box::new(value.into().0),
        })
    }

    /**
    Create a buffer for an enum variant with unnamed fields, like `A::B(T, U)`.
    */
    pub fn tuple_variant(
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        fields: impl IntoIterator<Item = Ref<'a>>,
    ) -> Self {
        Ref(Value::TupleVariant {
            name,
            variant_index,
            variant,
            fields: fields
                .into_iter()
                .map(|v| v.0)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
    }

    /**
    Create a buffer for an enum variant with named fields, like `A::B { a: T, b: U }`.
    */
    pub fn record_struct_variant(
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        fields: impl IntoIterator<Item = (&'static str, Ref<'a>)>,
    ) -> Self {
        Ref(Value::StructVariant {
            name,
            variant_index,
            variant,
            fields: fields
                .into_iter()
                .map(|(k, v)| (k, v.0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })
    }

    /**
    Create a buffer for a sequence.
    */
    pub fn seq(fields: impl IntoIterator<Item = Ref<'a>>) -> Self {
        Ref(Value::Seq(
            fields
                .into_iter()
                .map(|v| v.0)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ))
    }

    /**
    Create a buffer for a map.
    */
    pub fn map(fields: impl IntoIterator<Item = (Ref<'a>, Ref<'a>)>) -> Self {
        Ref(Value::Map(
            fields
                .into_iter()
                .map(|(k, v)| (k.0, v.0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Value<'a> {
    Unit,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
    Bool(bool),
    Char(char),
    Str(Box<str>),
    BorrowedStr(&'a str),
    Bytes(Box<[u8]>),
    BorrowedBytes(&'a [u8]),
    None,
    Some(Box<Value<'a>>),
    UnitStruct {
        name: &'static str,
    },
    NewtypeStruct {
        name: &'static str,
        value: Box<Value<'a>>,
    },
    Struct {
        name: &'static str,
        fields: Box<[(&'static str, Value<'a>)]>,
    },
    Tuple(Box<[Value<'a>]>),
    TupleStruct {
        name: &'static str,
        fields: Box<[Value<'a>]>,
    },
    UnitVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    },
    NewtypeVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: Box<Value<'a>>,
    },
    TupleVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        fields: Box<[Value<'a>]>,
    },
    StructVariant {
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        fields: Box<[(&'static str, Value<'a>)]>,
    },
    Seq(Box<[Value<'a>]>),
    Map(Box<[(Value<'a>, Value<'a>)]>),
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use alloc::borrow::{Cow, ToOwned};
    use serde::{
        de::{Deserializer, IntoDeserializer, Visitor},
        ser::SerializeMap,
        Deserialize, Serialize,
    };
    use serde_test::Token;

    use serde_derive::{Deserialize, Serialize};

    use super::*;

    #[test]
    fn consistency() {
        test_case(
            Input::new(()),
            Input::new(Ref::unit()),
            Tokens::new(&[Token::Unit]),
            None,
        );

        test_case(
            Input::new(true),
            Input::new(Ref::bool(true)),
            Tokens::new(&[Token::Bool(true)]),
            None,
        );

        test_case(
            Input::new('a'),
            Input::new(Ref::char('a')),
            Tokens::new(&[Token::Char('a')]),
            None,
        );

        test_case(
            Input::new(1u8),
            Input::new(Ref::u8(1)),
            Tokens::new(&[Token::U8(1)]),
            None,
        );
        test_case(
            Input::new(1u16),
            Input::new(Ref::u16(1)),
            Tokens::new(&[Token::U16(1)]),
            None,
        );
        test_case(
            Input::new(1u32),
            Input::new(Ref::u32(1)),
            Tokens::new(&[Token::U32(1)]),
            None,
        );
        test_case(
            Input::new(1u64),
            Input::new(Ref::u64(1)),
            Tokens::new(&[Token::U64(1)]),
            None,
        );

        test_case(
            Input::new(-1i8),
            Input::new(Ref::i8(-1)),
            Tokens::new(&[Token::I8(-1)]),
            None,
        );
        test_case(
            Input::new(-1i16),
            Input::new(Ref::i16(-1)),
            Tokens::new(&[Token::I16(-1)]),
            None,
        );
        test_case(
            Input::new(-1i32),
            Input::new(Ref::i32(-1)),
            Tokens::new(&[Token::I32(-1)]),
            None,
        );
        test_case(
            Input::new(-1i64),
            Input::new(Ref::i64(-1)),
            Tokens::new(&[Token::I64(-1)]),
            None,
        );

        i128_test_case(1u128, Ref::u128(1));
        i128_test_case(-1i128, Ref::i128(-1));

        test_case(
            Input::new(1f32),
            Input::new(Ref::f32(1.0)),
            Tokens::new(&[Token::F32(1.0)]),
            None,
        );
        test_case(
            Input::new(1f64),
            Input::new(Ref::f64(1.0)),
            Tokens::new(&[Token::F64(1.0)]),
            None,
        );

        test_case(
            Input::new(Str(Cow::Borrowed("a string"))),
            Input::new(Ref::str("a string")),
            Tokens::new(&[Token::BorrowedStr("a string")]),
            Tokens::new(&[Token::Str("a string")]),
        );

        test_case(
            Input::new(Str(Cow::Owned("a string".to_owned()))),
            Input::new(Ref::owned_str("a string")),
            Tokens::new(&[Token::Str("a string")]),
            None,
        );

        test_case(
            Input::new(Bytes(Cow::Borrowed(b"a string"))),
            Input::new(Ref::bytes(b"a string")),
            Tokens::new(&[Token::BorrowedBytes(b"a string")]),
            Tokens::new(&[Token::Bytes(b"a string")]),
        );

        test_case(
            Input::new(Bytes(Cow::Owned(b"a string".to_vec()))),
            Input::new(Ref::owned_bytes(b"a string" as &[u8])),
            Tokens::new(&[Token::Bytes(b"a string")]),
            None,
        );

        test_case(
            Input::new(None::<()>),
            Input::new(Ref::none()),
            Tokens::new(&[Token::None]),
            None,
        );
        test_case(
            Input::new(Some(())),
            Input::new(Ref::some(Ref::unit())),
            Tokens::new(&[Token::Some, Token::Unit]),
            None,
        );

        test_case(
            Input::new(UnitStruct),
            Input::new(Ref::unit_struct("UnitStruct")),
            Tokens::new(&[Token::UnitStruct { name: "UnitStruct" }]),
            None,
        );

        test_case(
            Input::new(Enum::UnitVariant),
            Input::new(Ref::unit_variant("Enum", 0, "UnitVariant")),
            Tokens::new(&[Token::UnitVariant {
                name: "Enum",
                variant: "UnitVariant",
            }]),
            None,
        );

        test_case(
            Input::new(Enum::NewtypeVariant(())),
            Input::new(Ref::newtype_variant(
                "Enum",
                1,
                "NewtypeVariant",
                Ref::unit(),
            )),
            Tokens::new(&[
                Token::NewtypeVariant {
                    name: "Enum",
                    variant: "NewtypeVariant",
                },
                Token::Unit,
            ]),
            None,
        );

        test_case(
            Input::new(Enum::TupleVariant((), ())),
            Input::new(Ref::tuple_variant(
                "Enum",
                2,
                "TupleVariant",
                alloc::vec![Ref::unit(), Ref::unit()],
            )),
            Tokens::new(&[
                Token::TupleVariant {
                    name: "Enum",
                    variant: "TupleVariant",
                    len: 2,
                },
                Token::Unit,
                Token::Unit,
                Token::TupleVariantEnd,
            ]),
            None,
        );

        test_case(
            Input::new(Enum::StructVariant { a: (), b: () }),
            Input::new(Ref::record_struct_variant(
                "Enum",
                3,
                "StructVariant",
                alloc::vec![("a", Ref::unit()), ("b", Ref::unit())],
            )),
            Tokens::new(&[
                Token::StructVariant {
                    name: "Enum",
                    variant: "StructVariant",
                    len: 2,
                },
                Token::Str("a"),
                Token::Unit,
                Token::Str("b"),
                Token::Unit,
                Token::StructVariantEnd,
            ]),
            None,
        );

        test_case(
            Input::new(((), ())),
            Input::new(Ref::tuple(alloc::vec![Ref::unit(), Ref::unit()])),
            Tokens::new(&[
                Token::Tuple { len: 2 },
                Token::Unit,
                Token::Unit,
                Token::TupleEnd,
            ]),
            None,
        );

        test_case(
            Input::new(TupleStruct((), ())),
            Input::new(Ref::tuple_struct(
                "TupleStruct",
                alloc::vec![Ref::unit(), Ref::unit()],
            )),
            Tokens::new(&[
                Token::TupleStruct {
                    name: "TupleStruct",
                    len: 2,
                },
                Token::Unit,
                Token::Unit,
                Token::TupleStructEnd,
            ]),
            None,
        );

        test_case(
            Input::new(Struct { a: (), b: () }),
            Input::new(Ref::record_struct(
                "Struct",
                alloc::vec![("a", Ref::unit()), ("b", Ref::unit())],
            )),
            Tokens::new(&[
                Token::Struct {
                    name: "Struct",
                    len: 2,
                },
                Token::Str("a"),
                Token::Unit,
                Token::Str("b"),
                Token::Unit,
                Token::StructEnd,
            ]),
            None,
        );

        test_case(
            Input::new(NewtypeStruct(())),
            Input::new(Ref::newtype_struct("NewtypeStruct", Ref::unit())),
            Tokens::new(&[
                Token::NewtypeStruct {
                    name: "NewtypeStruct",
                },
                Token::Unit,
            ]),
            None,
        );

        test_case(
            Input::new(alloc::vec![(), ()]),
            Input::new(Ref::seq(alloc::vec![Ref::unit(), Ref::unit()])),
            Tokens::new(&[
                Token::Seq { len: Some(2) },
                Token::Unit,
                Token::Unit,
                Token::SeqEnd,
            ]),
            None,
        );

        test_case(
            Input::new(Map(alloc::vec![
                (Str(Cow::Borrowed("a")), ()),
                (Str(Cow::Borrowed("b")), ())
            ])),
            Input::new(Ref::map(alloc::vec![
                (Ref::str("a"), Ref::unit()),
                (Ref::str("b"), Ref::unit())
            ])),
            Tokens::new(&[
                Token::Map { len: Some(2) },
                Token::Str("a"),
                Token::Unit,
                Token::Str("b"),
                Token::Unit,
                Token::MapEnd,
            ]),
            None,
        );
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Input<S> {
        value: S,
    }

    type Tokens<'de> = Input<&'de [Token]>;

    impl<S> Input<S> {
        fn new(value: S) -> Self {
            Input { value }
        }
    }

    fn test_case<'de, S: Serialize + Deserialize<'de> + PartialEq + fmt::Debug + Clone>(
        t: Input<S>,
        ref_buf: Input<Ref<'de>>,
        ref_tokens: Tokens<'de>,
        owned_tokens: impl Into<Option<Tokens<'de>>>,
    ) {
        let owned_tokens: Tokens<'de> = owned_tokens.into().unwrap_or(ref_tokens);

        serde_test::assert_ser_tokens(&t.value, ref_tokens.value);

        // T::Serialize -> Owned
        let t_to_owned = Input {
            value: t.value.serialize(Serializer::new()).unwrap(),
        };

        serde_test::assert_ser_tokens(&ref_buf.value, ref_tokens.value);
        serde_test::assert_ser_tokens(&t_to_owned.value, owned_tokens.value);

        // {Ref, Owned}::IntoDeserializer -> T
        let ref_to_t = Input {
            value: S::deserialize(ref_buf.value.into_deserializer()).unwrap(),
        };
        let owned_to_t = Input {
            value: S::deserialize(t_to_owned.value.into_deserializer()).unwrap(),
        };

        assert_eq!(t, ref_to_t);
        assert_eq!(t, owned_to_t);
    }

    fn i128_test_case<'de, T: Serialize + Deserialize<'de> + PartialEq + fmt::Debug>(
        v: T,
        ref_buf: Ref<'de>,
    ) {
        let t_to_owned = v.serialize(Serializer::new()).unwrap();

        assert_eq!(ref_buf.0, t_to_owned.0);

        let owned_to_t = T::deserialize(t_to_owned.into_deserializer()).unwrap();

        assert_eq!(v, owned_to_t);
    }

    #[derive(Debug, PartialEq, Clone)]
    struct Str<'a>(Cow<'a, str>);

    impl<'a> Serialize for Str<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(&self.0)
        }
    }

    impl<'de> Deserialize<'de> for Str<'de> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let v = deserializer.deserialize_str(StrVisitor(PhantomData))?;

            Ok(Str(v))
        }
    }

    struct StrVisitor<'de>(PhantomData<Cow<'de, str>>);

    impl<'de> Visitor<'de> for StrVisitor<'de> {
        type Value = Cow<'de, str>;

        fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
            write!(formatter, "a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Cow::Owned(v.to_owned()))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Cow::Owned(v))
        }

        fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Cow::Borrowed(v))
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    struct Bytes<'a>(Cow<'a, [u8]>);

    impl<'a> Serialize for Bytes<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_bytes(&self.0)
        }
    }

    impl<'de> Deserialize<'de> for Bytes<'de> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let v = deserializer.deserialize_bytes(BytesVisitor(PhantomData))?;

            Ok(Bytes(v))
        }
    }
    struct BytesVisitor<'de>(PhantomData<Cow<'de, [u8]>>);

    impl<'de> Visitor<'de> for BytesVisitor<'de> {
        type Value = Cow<'de, [u8]>;

        fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
            write!(formatter, "a byte string")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Cow::Owned(v.to_owned()))
        }

        fn visit_byte_buf<E>(self, v: alloc::vec::Vec<u8>) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Cow::Owned(v))
        }

        fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Cow::Borrowed(v))
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    struct UnitStruct;

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    struct TupleStruct((), ());

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    struct Struct {
        a: (),
        b: (),
    }

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    struct NewtypeStruct(());

    #[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
    enum Enum {
        UnitVariant,
        NewtypeVariant(()),
        TupleVariant((), ()),
        StructVariant { a: (), b: () },
    }

    #[derive(PartialEq, Clone, Debug)]
    struct Map<'a>(Vec<(Str<'a>, ())>);

    impl<'a> Serialize for Map<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut serializer = serializer.serialize_map(Some(self.0.len()))?;

            for (k, v) in &*self.0 {
                serializer.serialize_key(k)?;
                serializer.serialize_value(v)?;
            }

            serializer.end()
        }
    }

    impl<'de> Deserialize<'de> for Map<'de> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_map(MapVisitor(PhantomData))
        }
    }

    struct MapVisitor<'de>(PhantomData<Map<'de>>);

    impl<'de> Visitor<'de> for MapVisitor<'de> {
        type Value = Map<'de>;

        fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
            write!(formatter, "a map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut de = Vec::new();

            while let Some(k) = map.next_key()? {
                let v = map.next_value()?;

                de.push((k, v));
            }

            Ok(Map(de))
        }
    }
}
