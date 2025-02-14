use crate::ber::bytes_to_u64;
use crate::error::BerError;
use crate::oid::Oid;
use std::convert::AsRef;
use std::convert::From;
use std::ops::Index;
use std::vec::Vec;

/// Defined in X.680 section 8.4
/// X.690 doesn't specify the maxmimum tag size so we're assuming that people
/// aren't going to need anything more than a u32.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BerTag(pub u32);

newtype_enum! {
impl debug BerTag {
    EndOfContent = 0x0,
    Boolean = 0x1,
    Integer = 0x2,
    BitString = 0x3,
    OctetString = 0x4,
    Null = 0x05,
    Oid = 0x06,
    ObjDescriptor = 0x07,
    External = 0x08,
    RealType = 0x09,
    Enumerated = 0xa,
    EmbeddedPdv = 0xb,
    Utf8String = 0xc,
    RelativeOid = 0xd,

    Sequence = 0x10,
    Set = 0x11,
    NumericString = 0x12,
    PrintableString = 0x13,
    T61String = 0x14,

    Ia5String = 0x16,
    UtcTime = 0x17,
    GeneralizedTime = 0x18,

    GeneralString = 27, // 0x1b

    BmpString = 0x1e,

    Invalid = 0xff,
}
}

/// Representation of a DER-encoded (X.690) object
#[derive(Debug, Clone, PartialEq)]
pub struct BerObject<'a> {
    pub class: u8,
    pub structured: u8,
    pub tag: BerTag,

    pub content: BerObjectContent<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BerObjectHeader {
    pub class: u8,
    pub structured: u8,
    pub tag: BerTag,
    pub len: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BerObjectContent<'a> {
    EndOfContent,
    Boolean(bool),
    Integer(&'a [u8]),
    BitString(u8, BitStringObject<'a>),
    OctetString(&'a [u8]),
    Null,
    Enum(u64),
    OID(Oid),
    RelativeOID(Oid),
    NumericString(&'a [u8]),
    PrintableString(&'a [u8]),
    IA5String(&'a [u8]),
    UTF8String(&'a [u8]),
    T61String(&'a [u8]),

    BmpString(&'a [u8]),

    Sequence(Vec<BerObject<'a>>),
    Set(Vec<BerObject<'a>>),

    UTCTime(&'a [u8]),
    GeneralizedTime(&'a [u8]),

    GeneralString(&'a [u8]),

    ContextSpecific(BerTag, Option<Box<BerObject<'a>>>),
    Unknown(BerTag, &'a [u8]),
}

impl BerObjectHeader {
    /// Test if object class is Universal
    #[inline]
    pub fn is_universal(&self) -> bool {
        self.class == 0
    }
    /// Test if object class is Application
    #[inline]
    pub fn is_application(&self) -> bool {
        self.class == 0b01
    }
    /// Test if object class is Context-specific
    #[inline]
    pub fn is_contextspecific(&self) -> bool {
        self.class == 0b10
    }
    /// Test if object class is Private
    #[inline]
    pub fn is_private(&self) -> bool {
        self.class == 0b11
    }

    /// Test if object is primitive
    #[inline]
    pub fn is_primitive(&self) -> bool {
        self.structured == 0
    }
    /// Test if object is constructed
    #[inline]
    pub fn is_constructed(&self) -> bool {
        self.structured == 1
    }
}

impl<'a> BerObject<'a> {
    /// Build a BerObject from a header and content.
    /// Note: values are not checked, so the tag can be different from the real content, or flags
    /// can be invalid.
    pub fn from_header_and_content(hdr: BerObjectHeader, c: BerObjectContent) -> BerObject {
        BerObject {
            class: hdr.class,
            structured: hdr.structured,
            tag: hdr.tag,
            content: c,
        }
    }
    /// Build a BerObject from its content, using default flags (no class, correct tag,
    /// and structured flag set only for Set and Sequence)
    pub fn from_obj(c: BerObjectContent) -> BerObject {
        let class = 0;
        let tag = c.tag();
        let structured = match tag {
            BerTag::Sequence | BerTag::Set => 1,
            _ => 0,
        };
        BerObject {
            class,
            structured,
            tag,
            content: c,
        }
    }

    /// Build a DER integer object from a slice containing an encoded integer
    pub fn from_int_slice(i: &'a [u8]) -> BerObject<'a> {
        BerObject {
            class: 0,
            structured: 0,
            tag: BerTag::Integer,
            content: BerObjectContent::Integer(i),
        }
    }

    /// Build a DER sequence object from a vector of DER objects
    pub fn from_seq(l: Vec<BerObject>) -> BerObject {
        BerObject::from_obj(BerObjectContent::Sequence(l))
    }

    /// Build a DER set object from a vector of DER objects
    pub fn from_set(l: Vec<BerObject>) -> BerObject {
        BerObject::from_obj(BerObjectContent::Set(l))
    }

    /// Attempt to read integer value from DER object.
    /// This can fail if the object is not an integer, or if it is too large.
    ///
    /// ```rust
    /// # extern crate der_parser;
    /// # use der_parser::ber::{BerObject,BerObjectContent};
    /// # fn main() {
    /// let der_int  = BerObject::from_int_slice(b"\x01\x00\x01");
    /// assert_eq!(
    ///     der_int.as_u64(),
    ///     Ok(0x10001)
    /// );
    /// # }
    /// ```
    pub fn as_u64(&self) -> Result<u64, BerError> {
        self.content.as_u64()
    }

    /// Attempt to read integer value from DER object.
    /// This can fail if the object is not an integer, or if it is too large.
    ///
    /// ```rust
    /// # extern crate der_parser;
    /// # use der_parser::ber::{BerObject,BerObjectContent};
    /// # fn main() {
    /// let der_int  = BerObject::from_obj(BerObjectContent::Integer(b"\x01\x00\x01"));
    /// assert_eq!(
    ///     der_int.as_u32(),
    ///     Ok(0x10001)
    /// );
    /// # }
    /// ```
    pub fn as_u32(&self) -> Result<u32, BerError> {
        self.content.as_u32()
    }

    /// Attempt to read integer value from DER object.
    /// This can fail if the object is not a boolean.
    pub fn as_bool(&self) -> Result<bool, BerError> {
        self.content.as_bool()
    }

    /// Attempt to read an OID value from DER object.
    /// This can fail if the object is not an OID.
    ///
    /// Note that this function returns a reference to the OID. To get an owned value,
    /// use [`as_oid_val`](struct.BerObject.html#method.as_oid_val)
    pub fn as_oid(&self) -> Result<&Oid, BerError> {
        self.content.as_oid()
    }

    /// Attempt to read an OID value from DER object.
    /// This can fail if the object is not an OID.
    pub fn as_oid_val(&self) -> Result<Oid, BerError> {
        self.content.as_oid_val()
    }

    /// Attempt to read the content from a context-specific DER object.
    /// This can fail if the object is not context-specific.
    ///
    /// Note: the object is cloned.
    pub fn as_context_specific(&self) -> Result<(BerTag, Option<Box<BerObject<'a>>>), BerError> {
        self.content.as_context_specific()
    }

    /// Attempt to read a reference to a BitString value from DER object.
    /// This can fail if the object is not an BitString.
    ///
    /// Note that this function returns a reference to the BitString. To get an owned value,
    /// use [`as_bitstring`](struct.BerObject.html#method.as_bitstring)
    pub fn as_bitstring_ref(&self) -> Result<&BitStringObject, BerError> {
        self.content.as_bitstring_ref()
    }

    /// Attempt to read a BitString value from DER object.
    /// This can fail if the object is not an BitString.
    pub fn as_bitstring(&'a self) -> Result<BitStringObject<'a>, BerError> {
        self.content.as_bitstring()
    }

    /// Attempt to extract the list of objects from a DER sequence.
    /// This can fail if the object is not a sequence.
    pub fn as_sequence(&self) -> Result<&Vec<BerObject<'a>>, BerError> {
        self.content.as_sequence()
    }

    /// Attempt to extract the list of objects from a DER set.
    /// This can fail if the object is not a set.
    pub fn as_set(&self) -> Result<&Vec<BerObject<'a>>, BerError> {
        self.content.as_set()
    }

    /// Attempt to get the content from a DER object, as a slice.
    /// This can fail if the object does not contain a type directly equivalent to a slice (e.g a
    /// sequence).
    /// This function mostly concerns string types, integers, or unknown DER objects.
    pub fn as_slice(&self) -> Result<&'a [u8], BerError> {
        self.content.as_slice()
    }

    /// Test if object class is Universal
    pub fn is_universal(&self) -> bool {
        self.class == 0
    }
    /// Test if object class is Application
    pub fn is_application(&self) -> bool {
        self.class == 0b01
    }
    /// Test if object class is Context-specific
    pub fn is_contextspecific(&self) -> bool {
        self.class == 0b10
    }
    /// Test if object class is Private
    pub fn is_private(&self) -> bool {
        self.class == 0b11
    }

    /// Test if object is primitive
    pub fn is_primitive(&self) -> bool {
        self.structured == 0
    }
    /// Test if object is constructed
    pub fn is_constructed(&self) -> bool {
        self.structured == 1
    }
}

/// Build a DER object from an OID.
impl<'a> From<Oid> for BerObject<'a> {
    fn from(oid: Oid) -> BerObject<'a> {
        BerObject::from_obj(BerObjectContent::OID(oid))
    }
}

/// Build a DER object from a BerObjectContent.
impl<'a> From<BerObjectContent<'a>> for BerObject<'a> {
    fn from(obj: BerObjectContent<'a>) -> BerObject<'a> {
        BerObject::from_obj(obj)
    }
}

impl<'a> BerObjectContent<'a> {
    pub fn as_u64(&self) -> Result<u64, BerError> {
        match *self {
            BerObjectContent::Integer(i) => bytes_to_u64(i),
            BerObjectContent::Enum(i) => Ok(i as u64),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_u32(&self) -> Result<u32, BerError> {
        match *self {
            BerObjectContent::Integer(i) => bytes_to_u64(i).and_then(|x| {
                if x > std::u32::MAX as u64 {
                    Err(BerError::IntegerTooLarge)
                } else {
                    Ok(x as u32)
                }
            }),
            BerObjectContent::Enum(i) => {
                if i > std::u32::MAX as u64 {
                    Err(BerError::IntegerTooLarge)
                } else {
                    Ok(i as u32)
                }
            }
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_bool(&self) -> Result<bool, BerError> {
        match *self {
            BerObjectContent::Boolean(b) => Ok(b),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_oid(&self) -> Result<&Oid, BerError> {
        match *self {
            BerObjectContent::OID(ref o) => Ok(o),
            BerObjectContent::RelativeOID(ref o) => Ok(o),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_oid_val(&self) -> Result<Oid, BerError> {
        match *self {
            BerObjectContent::OID(ref o) => Ok(o.to_owned()),
            BerObjectContent::RelativeOID(ref o) => Ok(o.to_owned()),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_context_specific(&self) -> Result<(BerTag, Option<Box<BerObject<'a>>>), BerError> {
        match *self {
            BerObjectContent::ContextSpecific(u, ref o) => Ok((u, o.clone())),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_bitstring_ref(&self) -> Result<&BitStringObject, BerError> {
        match *self {
            BerObjectContent::BitString(_, ref b) => Ok(b),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_bitstring(&'a self) -> Result<BitStringObject<'a>, BerError> {
        match *self {
            BerObjectContent::BitString(_, ref b) => Ok(b.to_owned()),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_sequence(&self) -> Result<&Vec<BerObject<'a>>, BerError> {
        match *self {
            BerObjectContent::Sequence(ref s) => Ok(s),
            _ => Err(BerError::BerTypeError),
        }
    }

    pub fn as_set(&self) -> Result<&Vec<BerObject<'a>>, BerError> {
        match *self {
            BerObjectContent::Set(ref s) => Ok(s),
            _ => Err(BerError::BerTypeError),
        }
    }

    #[rustfmt::skip]
    pub fn as_slice(&self) -> Result<&'a [u8],BerError> {
        match *self {
            BerObjectContent::Integer(s) |
            BerObjectContent::BitString(_,BitStringObject{data:s}) |
            BerObjectContent::OctetString(s) |
            BerObjectContent::NumericString(s) |
            BerObjectContent::PrintableString(s) |
            BerObjectContent::IA5String(s) |
            BerObjectContent::UTF8String(s) |
            BerObjectContent::T61String(s) |
            BerObjectContent::BmpString(s) |
            BerObjectContent::GeneralString(s) |
            BerObjectContent::Unknown(_,s) => Ok(s),
            _ => Err(BerError::BerTypeError),
        }
    }

    #[rustfmt::skip]
    pub fn tag(&self) -> BerTag {
        match *self {
            BerObjectContent::EndOfContent         => BerTag::EndOfContent,
            BerObjectContent::Boolean(_)           => BerTag::Boolean,
            BerObjectContent::Integer(_)           => BerTag::Integer,
            BerObjectContent::BitString(_,_)       => BerTag::BitString,
            BerObjectContent::OctetString(_)       => BerTag::OctetString,
            BerObjectContent::Null                 => BerTag::Null,
            BerObjectContent::Enum(_)              => BerTag::Enumerated,
            BerObjectContent::OID(_)               => BerTag::Oid,
            BerObjectContent::NumericString(_)     => BerTag::NumericString,
            BerObjectContent::PrintableString(_)   => BerTag::PrintableString,
            BerObjectContent::IA5String(_)         => BerTag::Ia5String,
            BerObjectContent::UTF8String(_)        => BerTag::Utf8String,
            BerObjectContent::RelativeOID(_)       => BerTag::RelativeOid,
            BerObjectContent::T61String(_)         => BerTag::T61String,
            BerObjectContent::BmpString(_)         => BerTag::BmpString,
            BerObjectContent::Sequence(_)          => BerTag::Sequence,
            BerObjectContent::Set(_)               => BerTag::Set,
            BerObjectContent::UTCTime(_)           => BerTag::UtcTime,
            BerObjectContent::GeneralizedTime(_)   => BerTag::GeneralizedTime,
            BerObjectContent::GeneralString(_)     => BerTag::GeneralString,
            BerObjectContent::ContextSpecific(x,_) |
            BerObjectContent::Unknown(x,_)         => x,
        }
    }
}

#[cfg(feature = "bigint")]
mod bigint {
    use super::{BerObject, BerObjectContent};
    use num_bigint::{BigInt, BigUint, Sign};

    impl<'a> BerObject<'a> {
        pub fn as_bigint(&self) -> Option<BigInt> {
            match self.content {
                BerObjectContent::Integer(s) => Some(BigInt::from_bytes_be(Sign::Plus, s)),
                _ => None,
            }
        }

        pub fn as_biguint(&self) -> Option<BigUint> {
            match self.content {
                BerObjectContent::Integer(s) => Some(BigUint::from_bytes_be(s)),
                _ => None,
            }
        }
    }
}

// This is a consuming iterator
impl<'a> IntoIterator for BerObject<'a> {
    type Item = BerObject<'a>;
    type IntoIter = BerObjectIntoIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        // match self {
        //     BerObjectContent::Sequence(ref v) => (),
        //     _ => (),
        // };
        BerObjectIntoIterator { val: self, idx: 0 }
    }
}

pub struct BerObjectIntoIterator<'a> {
    val: BerObject<'a>,
    idx: usize,
}

impl<'a> Iterator for BerObjectIntoIterator<'a> {
    type Item = BerObject<'a>;
    fn next(&mut self) -> Option<BerObject<'a>> {
        // let result = if self.idx < self.vec.len() {
        //     Some(self.vec[self.idx].clone())
        // } else {
        //     None
        // };
        let res = match self.val.content {
            BerObjectContent::Sequence(ref v) if self.idx < v.len() => Some(v[self.idx].clone()),
            BerObjectContent::Set(ref v) if self.idx < v.len() => Some(v[self.idx].clone()),
            _ => {
                if self.idx == 0 {
                    Some(self.val.clone())
                } else {
                    None
                }
            }
        };
        self.idx += 1;
        res
    }
}

// impl<'a> Iterator for BerObjectContent<'a> {
//     type Item = BerObjectContent<'a>;
//
//     fn next(&mut self) -> Option<BerObjectContent<'a>> {
//         None
//     }
// }

pub struct BerObjectRefIterator<'a> {
    obj: &'a BerObject<'a>,
    idx: usize,
}

impl<'a> Iterator for BerObjectRefIterator<'a> {
    type Item = &'a BerObject<'a>;
    fn next(&mut self) -> Option<&'a BerObject<'a>> {
        let res = match (*self.obj).content {
            BerObjectContent::Sequence(ref v) if self.idx < v.len() => Some(&v[self.idx]),
            BerObjectContent::Set(ref v) if self.idx < v.len() => Some(&v[self.idx]),
            _ => None,
        };
        self.idx += 1;
        res
    }
}

impl<'a> BerObject<'a> {
    pub fn ref_iter(&'a self) -> BerObjectRefIterator<'a> {
        BerObjectRefIterator { obj: self, idx: 0 }
    }
}

impl<'a> Index<usize> for BerObject<'a> {
    type Output = BerObject<'a>;

    fn index(&self, idx: usize) -> &BerObject<'a> {
        match (*self).content {
            BerObjectContent::Sequence(ref v) if idx < v.len() => &v[idx],
            BerObjectContent::Set(ref v) if idx < v.len() => &v[idx],
            _ => panic!("Try to index BerObjectContent which is not structured"),
        }
        // XXX the following
        // self.ref_iter().nth(idx).unwrap()
        // fails with:
        // error: cannot infer an appropriate lifetime for autoref due to conflicting requirements [E0495]
        // self.ref_iter().nth(idx).unwrap()
    }
}

/// BitString wrapper
#[derive(Clone, Debug, PartialEq)]
pub struct BitStringObject<'a> {
    pub data: &'a [u8],
}

impl<'a> BitStringObject<'a> {
    /// Test if bit `bitnum` is set
    pub fn is_set(&self, bitnum: usize) -> bool {
        let byte_pos = bitnum / 8;
        if byte_pos >= self.data.len() {
            return false;
        }
        let b = 7 - (bitnum % 8);
        (self.data[byte_pos] & (1 << b)) != 0
    }
}

impl<'a> AsRef<[u8]> for BitStringObject<'a> {
    fn as_ref(&self) -> &[u8] {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use crate::ber::*;
    use crate::oid::*;

    #[test]
    fn test_der_as_u64() {
        let der_obj = BerObject::from_int_slice(b"\x01\x00\x02");
        assert_eq!(der_obj.as_u64(), Ok(0x10002));
    }

    #[test]
    fn test_der_seq_iter() {
        let der_obj = BerObject::from_obj(BerObjectContent::Sequence(vec![
            BerObject::from_int_slice(b"\x01\x00\x01"),
            BerObject::from_int_slice(b"\x01\x00\x00"),
        ]));
        let expected_values = vec![
            BerObject::from_int_slice(b"\x01\x00\x01"),
            BerObject::from_int_slice(b"\x01\x00\x00"),
        ];

        let mut idx = 0;
        // for v in res {
        //     debug!("v: {:?}", v);
        //     assert_eq!(v,expected_values[idx]);
        //     idx += 1;
        // }
        for v in der_obj.ref_iter() {
            // println!("v: {:?}", v);
            assert_eq!((*v), expected_values[idx]);
            idx += 1;
        }
    }

    #[test]
    fn test_der_from_oid() {
        let obj: BerObject = Oid::from(&[1, 2]).into();
        let expected = BerObject::from_obj(BerObjectContent::OID(Oid::from(&[1, 2])));

        assert_eq!(obj, expected);
    }

    #[test]
    fn test_der_bistringobject() {
        let obj = BitStringObject {
            data: &[0x0f, 0x00, 0x40],
        };
        assert!(!obj.is_set(0));
        assert!(obj.is_set(7));
        assert!(!obj.is_set(9));
        assert!(obj.is_set(17));
    }

    #[test]
    fn test_der_bistringobject_asref() {
        fn assert_equal<T: AsRef<[u8]>>(s: T, b: &[u8]) {
            assert_eq!(s.as_ref(), b);
        }
        let b: &[u8] = &[0x0f, 0x00, 0x40];
        let obj = BitStringObject { data: b };
        assert_equal(obj, b);
    }

    #[cfg(feature = "bigint")]
    #[test]
    fn test_der_to_bigint() {
        let obj = BerObject::from_obj(BerObjectContent::Integer(b"\x01\x00\x01"));
        let expected = ::num_bigint::BigInt::from(0x10001);

        assert_eq!(obj.as_bigint(), Some(expected));
    }

    #[cfg(feature = "bigint")]
    #[test]
    fn test_der_to_biguint() {
        let obj = BerObject::from_obj(BerObjectContent::Integer(b"\x01\x00\x01"));
        let expected = ::num_bigint::BigUint::from(0x10001 as u32);

        assert_eq!(obj.as_biguint(), Some(expected));
    }

}
