use std::{
    convert::TryInto,
    mem::{size_of, MaybeUninit},
    num::TryFromIntError,
    ptr::null,
};

use rust_guile::{
    scm_c_bytevector_length, scm_c_bytevector_ref, scm_c_bytevector_set_x, scm_c_make_bytevector,
    scm_char_p, scm_from_double, scm_from_int16, scm_from_int32, scm_from_int64, scm_from_int8,
    scm_from_uint16, scm_from_uint32, scm_from_uint64, scm_from_uint8, scm_from_utf8_stringn,
    scm_from_utf8_symboln, scm_gc_malloc, scm_integer_to_char, scm_is_bytevector,
    scm_is_signed_integer, scm_is_unsigned_integer, scm_real_p, scm_symbol_p, scm_symbol_to_string,
    scm_to_double, scm_to_int64, scm_to_int8, scm_to_uint32, scm_to_uint64, scm_to_utf32_stringn,
    scm_to_utf8_stringn, SCM,
};
use serde::{
    de,
    ser::{self, SerializeStruct, SerializeStructVariant, SerializeTuple},
};

#[derive(Debug)]
pub enum Error {
    ExpectedUnsignedInteger,
    ExpectedSignedInteger,
    IntegerOutOfBounds(TryFromIntError),
    ExpectedFloat,
    ExpectedChar,
    ExpectedStringOrSymbol,
    ExpectedByteVector,
    ExpectedList { n_elts: Option<usize> },
    ExpectedNil,
    ExpectedSymbol { sym: Option<String> },
    ExpectedAlist,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        todo!()
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        todo!()
    }
}

pub const SCM_BOOL_F: SCM = 0x4 as SCM;
pub const SCM_BOOL_T: SCM = 0x404 as SCM;
pub const SCM_UNSPECIFIED: SCM = 0x804 as SCM;
pub const SCM_EOL: SCM = 0x304 as SCM;

pub fn scm_is_true(scm: SCM) -> bool {
    !(scm == SCM_BOOL_F)
}

#[repr(C)]
struct ScmCell {
    car: SCM,
    cdr: SCM,
}

pub fn scm_cons(car: SCM, cdr: SCM) -> SCM {
    let cell = unsafe { scm_gc_malloc(size_of::<ScmCell>() as u64, null()) } as *mut ScmCell;
    unsafe {
        std::ptr::write(cell, ScmCell { car, cdr });
    }
    cell as SCM
}

pub enum ListSerializer {
    Empty,
    Heap { head: SCM, tail: SCM },
}

impl ser::SerializeSeq for ListSerializer {
    type Ok = SCM;

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = value.serialize(Serializer {
            strings_as_syms: false,
        })?;
        match self {
            ListSerializer::Empty => {
                let cell = scm_cons(value, SCM_EOL);
                *self = ListSerializer::Heap {
                    head: cell,
                    tail: cell,
                };
            }
            ListSerializer::Heap { head, tail } => {
                let cell = scm_cons(value, SCM_EOL);
                unsafe { std::ptr::write((*tail as *mut SCM).add(1), cell) };
                *tail = cell;
            }
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self {
            ListSerializer::Empty => SCM_EOL,
            ListSerializer::Heap { head, tail: _ } => head,
        })
    }
}

pub enum TupleSerializer {
    Empty,
    One(SCM),
    Heap { head: SCM, tail: SCM },
}

impl ser::SerializeTuple for TupleSerializer {
    type Ok = SCM;

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let value = value.serialize(Serializer {
            strings_as_syms: false,
        })?;
        match self {
            TupleSerializer::Empty => {
                *self = TupleSerializer::One(value);
            }
            TupleSerializer::One(old_value) => {
                let pair = scm_cons(*old_value, value);
                *self = TupleSerializer::Heap {
                    head: pair,
                    tail: pair,
                }
            }
            TupleSerializer::Heap { head: _, tail } => unsafe {
                let p_tail_cdr = (*tail as *mut SCM).add(1);
                let tail_cdr = std::ptr::read(p_tail_cdr);
                let new_pair = scm_cons(tail_cdr, value);
                std::ptr::write(p_tail_cdr, new_pair);
                *tail = new_pair;
            },
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self {
            TupleSerializer::Empty => panic!("Saw zero-element tuple, rather than \"Unit\""),
            TupleSerializer::One(v) => v,
            TupleSerializer::Heap { head, tail: _ } => head,
        })
    }
}

impl ser::SerializeTupleStruct for TupleSerializer {
    type Ok = SCM;

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeTuple>::end(self)
    }
}

impl ser::SerializeTupleVariant for TupleSerializer {
    type Ok = SCM;

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeTuple>::end(self)
    }
}

pub struct MapSerializer {
    list: SCM,
    new_key: Option<SCM>,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = SCM;

    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        assert!(self.new_key.is_none());
        self.new_key = Some(key.serialize(Serializer {
            strings_as_syms: true,
        })?);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let key = self.new_key.take().unwrap();
        let value = value.serialize(Serializer {
            strings_as_syms: false,
        })?;
        unsafe {
            let new_car = scm_cons(key, value);
            let new_list = scm_cons(new_car, self.list);
            self.list = new_list;
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        assert!(self.new_key.is_none());
        Ok(self.list)
    }
}

pub struct StructSerializer(SCM);

impl SerializeStruct for StructSerializer {
    type Ok = SCM;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        let new_car = unsafe {
            scm_cons(
                scm_from_utf8_symboln(std::mem::transmute(key.as_ptr()), key.len() as u64),
                value.serialize(Serializer {
                    strings_as_syms: false,
                })?,
            )
        };
        let new_list = scm_cons(new_car, self.0);
        self.0 = new_list;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.0)
    }
}

pub struct StructVariantSerializer {
    name_sym: SCM,
    inner: StructSerializer,
}

impl SerializeStructVariant for StructVariantSerializer {
    type Ok = SCM;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.inner.serialize_field(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let cdr = self.inner.end()?;
        Ok(scm_cons(self.name_sym, cdr))
    }
}

#[derive(Default)]
pub struct Serializer {
    strings_as_syms: bool,
}

impl ser::Serializer for Serializer {
    type Ok = SCM;
    type Error = Error;

    type SerializeSeq = ListSerializer;

    type SerializeTuple = TupleSerializer;

    type SerializeTupleStruct = TupleSerializer;

    type SerializeTupleVariant = TupleSerializer;

    type SerializeMap = MapSerializer;

    type SerializeStruct = StructSerializer;

    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(if v { SCM_BOOL_T } else { SCM_BOOL_F })
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_int8(v) })
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_int16(v) })
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_int32(v) })
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_int64(v) })
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_uint8(v) })
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_uint16(v) })
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_uint32(v) })
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_uint64(v) })
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_double(v as f64) })
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_double(v) })
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_integer_to_char(scm_from_uint64(v as u64)) })
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let func = if self.strings_as_syms {
            scm_from_utf8_symboln
        } else {
            scm_from_utf8_stringn
        };
        Ok(unsafe { (func)(std::mem::transmute(v.as_ptr()), v.len() as u64) })
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let bv = unsafe { scm_c_make_bytevector(v.len() as u64) };
        for (i, b) in v.iter().enumerate() {
            unsafe {
                scm_c_bytevector_set_x(bv, i as u64, *b);
            }
        }
        Ok(bv)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(SCM_EOL)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        Ok(scm_cons(value.serialize(self)?, SCM_EOL))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(SCM_EOL)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe { scm_from_utf8_symboln(std::mem::transmute(name.as_ptr()), name.len() as u64) })
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(unsafe {
            scm_from_utf8_symboln(std::mem::transmute(variant.as_ptr()), variant.len() as u64)
        })
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let car = unsafe {
            scm_from_utf8_symboln(std::mem::transmute(variant.as_ptr()), variant.len() as u64)
        };
        let cdr = value.serialize(self)?;
        Ok(scm_cons(car, cdr))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ListSerializer::Empty)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer::Empty)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleSerializer::Empty)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        let car = unsafe {
            scm_from_utf8_symboln(std::mem::transmute(variant.as_ptr()), variant.len() as u64)
        };
        Ok(TupleSerializer::One(car))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            list: SCM_EOL,
            new_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer(SCM_EOL))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        let name_sym = unsafe {
            scm_from_utf8_symboln(std::mem::transmute(variant.as_ptr()), variant.len() as u64)
        };
        Ok(StructVariantSerializer {
            name_sym,
            inner: StructSerializer(SCM_EOL),
        })
    }
}

pub struct Deserializer {
    pub scm: SCM,
}

pub fn try_scm_to_signed(scm: SCM) -> Option<i64> {
    unsafe { (scm_is_signed_integer(scm, i64::MIN, i64::MAX) != 0).then(|| scm_to_int64(scm)) }
}

pub fn try_scm_to_unsigned(scm: SCM) -> Option<u64> {
    unsafe { (scm_is_unsigned_integer(scm, u64::MIN, u64::MAX) != 0).then(|| scm_to_uint64(scm)) }
}

pub fn try_scm_to_double(scm: SCM) -> Option<f64> {
    unsafe { (scm_is_true(scm_real_p(scm))).then(|| scm_to_double(scm)) }
}

pub fn try_scm_to_char(scm: SCM) -> Option<char> {
    unsafe {
        (scm_is_true(scm_char_p(scm)))
            .then(|| scm_to_uint32(scm_integer_to_char(scm)))
            .map(|ch| ch.try_into().unwrap())
    }
}

pub fn scm_imp(scm: SCM) -> bool {
    (scm as usize) & 0x6 != 0
}

pub fn scm_nimp(scm: SCM) -> bool {
    !scm_imp(scm)
}

pub fn scm_typ7(scm: SCM) -> u8 {
    (unsafe { std::ptr::read(scm as *const usize) } as u8) & 0x7f
}

pub fn scm_is_string(scm: SCM) -> bool {
    scm_nimp(scm) && scm_typ7(scm) == 0x15
}

pub fn try_scm_to_sym(scm: SCM) -> Option<String> {
    unsafe {
        scm_is_true(scm_symbol_p(scm)).then(|| {
            let mut len = MaybeUninit::uninit();
            let data = scm_to_utf8_stringn(scm_symbol_to_string(scm), len.as_mut_ptr());
            let len = len.assume_init();
            String::from_raw_parts(
                std::mem::transmute(data),
                len.try_into().unwrap(),
                len.try_into().unwrap(),
            )
        })
    }
}

pub fn try_scm_to_string_or_sym(scm: SCM) -> Option<String> {
    unsafe {
        scm_is_string(scm)
            .then(|| {
                let mut len = MaybeUninit::uninit();
                let data = scm_to_utf8_stringn(scm, len.as_mut_ptr());
                let len = len.assume_init();
                String::from_raw_parts(
                    std::mem::transmute(data),
                    len.try_into().unwrap(),
                    len.try_into().unwrap(),
                )
            })
            .or_else(|| try_scm_to_sym(scm))
    }
}

pub fn try_scm_to_bytes(scm: SCM) -> Option<Vec<u8>> {
    unsafe {
        (scm_is_bytevector(scm) != 0).then(|| {
            let len = scm_c_bytevector_length(scm);
            let mut vec = Vec::with_capacity(len.try_into().unwrap());
            for i in 0..len {
                vec.push(scm_c_bytevector_ref(scm, i)); // Is there really no more performant way than the loop?
            }
            vec
        })
    }
}

unsafe fn scm_car_unchecked(scm: SCM) -> SCM {
    std::ptr::read(scm as *const SCM)
}

unsafe fn scm_cdr_unchecked(scm: SCM) -> SCM {
    std::ptr::read((scm as *const SCM).add(1))
}

fn scm_is_pair(scm: SCM) -> bool {
    unsafe {
        let raw = scm as usize;
        // See "Representation of scheme objects" in scm.h
        ((raw & 6) == 0) && ((std::ptr::read(scm as *const usize) & 1) == 0)
    }
}

pub fn try_scm_decons(scm: SCM) -> Option<(SCM, SCM)> {
    unsafe { scm_is_pair(scm).then(|| (scm_car_unchecked(scm), scm_cdr_unchecked(scm))) }
}

struct ListAccess {
    scm: SCM,
}

impl<'de> de::SeqAccess<'de> for ListAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.scm == SCM_EOL {
            Ok(None)
        } else {
            let (car, cdr) =
                try_scm_decons(self.scm).ok_or(Error::ExpectedList { n_elts: None })?;
            self.scm = cdr;
            seed.deserialize(Deserializer { scm: car }).map(Some)
        }
    }
}

struct TupleAccess {
    scm: Option<SCM>,
}

impl<'de> de::SeqAccess<'de> for TupleAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.scm {
            None => Ok(None),
            Some(scm) => {
                let next = match try_scm_decons(scm) {
                    Some((car, cdr)) => {
                        self.scm = Some(cdr);
                        car
                    }
                    None => {
                        self.scm = None;
                        scm
                    }
                };
                seed.deserialize(Deserializer { scm: next }).map(Some)
            }
        }
    }
}

struct AlistStructAccess {
    fields: Vec<(Option<SCM>, &'static str)>,
    scm: SCM,
}

impl<'de> de::SeqAccess<'de> for AlistStructAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.fields.is_empty() {
            Ok(None)
        } else {
            while self.fields.last().unwrap().0.is_none() {
                let (car, cdr) = try_scm_decons(self.scm).ok_or_else(|| Error::ExpectedSymbol {
                    sym: Some(self.fields.last().unwrap().1.to_string()),
                })?;

                self.scm = cdr;
                let (symbol, value) = try_scm_decons(car).ok_or(Error::ExpectedAlist)?;
                let symbol = try_scm_to_sym(symbol).ok_or(Error::ExpectedAlist)?;
                if let Some((maybe_old_part, _)) = self
                    .fields
                    .iter_mut()
                    .find(|(_, symbol_needle)| *symbol_needle == &symbol)
                {
                    if maybe_old_part.is_none() {
                        *maybe_old_part = Some(value);
                    }
                }
            }
            let piece = self.fields.pop().unwrap().0.unwrap();
            seed.deserialize(Deserializer { scm: piece }).map(Some)
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_bool(scm_is_true(self.scm))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: i8 = try_scm_to_signed(self.scm)
            .ok_or(Error::ExpectedSignedInteger)
            .and_then(|i| i.try_into().map_err(|e| Error::IntegerOutOfBounds(e)))?;
        visitor.visit_i8(val)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: i16 = try_scm_to_signed(self.scm)
            .ok_or(Error::ExpectedSignedInteger)
            .and_then(|i| i.try_into().map_err(|e| Error::IntegerOutOfBounds(e)))?;
        visitor.visit_i16(val)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: i32 = try_scm_to_signed(self.scm)
            .ok_or(Error::ExpectedSignedInteger)
            .and_then(|i| i.try_into().map_err(|e| Error::IntegerOutOfBounds(e)))?;
        visitor.visit_i32(val)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: i64 = try_scm_to_signed(self.scm).ok_or(Error::ExpectedSignedInteger)?;
        visitor.visit_i64(val)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: u8 = try_scm_to_unsigned(self.scm)
            .ok_or(Error::ExpectedUnsignedInteger)
            .and_then(|u| u.try_into().map_err(|e| Error::IntegerOutOfBounds(e)))?;
        visitor.visit_u8(val)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: u16 = try_scm_to_unsigned(self.scm)
            .ok_or(Error::ExpectedUnsignedInteger)
            .and_then(|u| u.try_into().map_err(|e| Error::IntegerOutOfBounds(e)))?;
        visitor.visit_u16(val)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: u32 = try_scm_to_unsigned(self.scm)
            .ok_or(Error::ExpectedUnsignedInteger)
            .and_then(|u| u.try_into().map_err(|e| Error::IntegerOutOfBounds(e)))?;
        visitor.visit_u32(val)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val: u64 = try_scm_to_unsigned(self.scm).ok_or(Error::ExpectedUnsignedInteger)?;
        visitor.visit_u64(val)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val = try_scm_to_double(self.scm).ok_or(Error::ExpectedFloat)? as f32;
        visitor.visit_f32(val)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val = try_scm_to_double(self.scm).ok_or(Error::ExpectedFloat)?;
        visitor.visit_f64(val)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val = try_scm_to_char(self.scm).ok_or(Error::ExpectedChar)?;
        visitor.visit_char(val)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val = try_scm_to_string_or_sym(self.scm).ok_or(Error::ExpectedStringOrSymbol)?;
        visitor.visit_string(val)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let val = try_scm_to_bytes(self.scm).ok_or(Error::ExpectedByteVector)?;
        visitor.visit_byte_buf(val)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.scm == SCM_EOL {
            visitor.visit_none()
        } else if let Some((car, SCM_EOL)) = try_scm_decons(self.scm) {
            visitor.visit_some(Deserializer { scm: car })
        } else {
            Err(Error::ExpectedList { n_elts: Some(1) })
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.scm == SCM_EOL {
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNil)
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let sym = try_scm_to_sym(self.scm).ok_or_else(|| Error::ExpectedSymbol {
            sym: Some(name.to_string()),
        })?;
        if name == &sym {
            visitor.visit_unit() // XXX check that this is correct
        } else {
            Err(Error::ExpectedSymbol {
                sym: Some(name.to_string()),
            })
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(ListAccess { scm: self.scm })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(TupleAccess {
            scm: Some(self.scm),
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let mut fields: Vec<_> = fields.iter().map(|f| (None, *f)).collect();
        fields.reverse();
        visitor.visit_seq(AlistStructAccess {
            fields,
            scm: self.scm,
        })
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }
}
