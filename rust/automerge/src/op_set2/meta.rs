use super::packer::{lebsize, ulebsize, MaybePackable, PackError, Packable, RleCursor, WriteOp};
use super::types::ScalarValue;

#[derive(Debug)]
pub(crate) enum ValueType {
    Null,
    False,
    True,
    Uleb,
    Leb,
    Float,
    String,
    Bytes,
    Counter,
    Timestamp,
    Unknown(u8),
}

#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd)]
pub(crate) struct ValueMeta(u64);

impl ValueMeta {
    pub(crate) fn type_code(&self) -> ValueType {
        let low_byte = (self.0 as u8) & 0b00001111;
        match low_byte {
            0 => ValueType::Null,
            1 => ValueType::False,
            2 => ValueType::True,
            3 => ValueType::Uleb,
            4 => ValueType::Leb,
            5 => ValueType::Float,
            6 => ValueType::String,
            7 => ValueType::Bytes,
            8 => ValueType::Counter,
            9 => ValueType::Timestamp,
            other => ValueType::Unknown(other),
        }
    }

    pub(crate) fn length(&self) -> usize {
        (self.0 >> 4) as usize
    }
}

impl From<u64> for ValueMeta {
    fn from(raw: u64) -> Self {
        ValueMeta(raw)
    }
}

impl<'a> From<ValueMeta> for WriteOp<'a> {
    fn from(v: ValueMeta) -> WriteOp<'static> {
        WriteOp::GroupUInt(v.0, v.length())
    }
}

impl From<&crate::ScalarValue> for ValueMeta {
    fn from(p: &crate::ScalarValue) -> Self {
        match p {
            crate::ScalarValue::Uint(i) => Self((ulebsize(*i) << 4) | 3),
            crate::ScalarValue::Int(i) => Self((lebsize(*i) << 4) | 4),
            crate::ScalarValue::Null => Self(0),
            crate::ScalarValue::Boolean(b) => Self(match b {
                false => 1,
                true => 2,
            }),
            crate::ScalarValue::Timestamp(i) => Self((lebsize(*i) << 4) | 9),
            crate::ScalarValue::F64(_) => Self((8 << 4) | 5),
            crate::ScalarValue::Counter(i) => Self((lebsize(i.start) << 4) | 8),
            crate::ScalarValue::Str(s) => Self(((s.as_bytes().len() as u64) << 4) | 6),
            crate::ScalarValue::Bytes(b) => Self(((b.len() as u64) << 4) | 7),
            crate::ScalarValue::Unknown { type_code, bytes } => {
                Self(((bytes.len() as u64) << 4) | (*type_code as u64))
            }
        }
    }
}

impl<'a> From<&'a ScalarValue<'a>> for ValueMeta {
    fn from(p: &'a ScalarValue<'a>) -> Self {
        match p {
            ScalarValue::Uint(i) => Self((ulebsize(*i) << 4) | 3),
            ScalarValue::Int(i) => Self((lebsize(*i) << 4) | 4),
            ScalarValue::Null => Self(0),
            ScalarValue::Boolean(b) => Self(match b {
                false => 1,
                true => 2,
            }),
            ScalarValue::Timestamp(i) => Self((lebsize(*i) << 4) | 9),
            ScalarValue::F64(_) => Self((8 << 4) | 5),
            ScalarValue::Counter(i) => Self((lebsize(*i) << 4) | 8),
            ScalarValue::Str(s) => Self(((s.as_bytes().len() as u64) << 4) | 6),
            ScalarValue::Bytes(b) => Self(((b.len() as u64) << 4) | 7),
            ScalarValue::Unknown { type_code, bytes } => {
                Self(((bytes.len() as u64) << 4) | (*type_code as u64))
            }
        }
    }
}

impl Packable for ValueMeta {
    type Unpacked<'a> = ValueMeta;
    type Owned = ValueMeta;

    fn group(item: ValueMeta) -> usize {
        item.length()
    }

    fn own(item: ValueMeta) -> ValueMeta {
        item
    }

    fn unpack(mut buff: &[u8]) -> Result<(usize, Self::Unpacked<'_>), PackError> {
        let start_len = buff.len();
        let val = leb128::read::unsigned(&mut buff)?;
        Ok((start_len - buff.len(), ValueMeta(val)))
    }
}

impl MaybePackable<ValueMeta> for ValueMeta {
    fn maybe_packable(&self) -> Option<ValueMeta> {
        Some(*self)
    }
}

impl MaybePackable<ValueMeta> for Option<ValueMeta> {
    fn maybe_packable(&self) -> Option<ValueMeta> {
        *self
    }
}

pub(crate) type MetaCursor = RleCursor<64, ValueMeta>;

#[cfg(test)]
mod tests {
    use super::super::packer::ColumnData;
    use super::*;

    #[test]
    fn column_data_meta_group() {
        let data = vec![
            ValueMeta(1),
            ValueMeta(6 + (30 << 4)),
            ValueMeta(6 + (10 << 4)),
            ValueMeta(3),
            ValueMeta(4),
        ];
        let mut col = ColumnData::<MetaCursor>::new();
        col.splice(0, 0, data);

        let mut iter = col.iter().with_group();

        let r = iter.next().unwrap();
        assert_eq!(r.item, Some(ValueMeta(1)));
        assert_eq!(r.group, 0);

        let r = iter.next().unwrap();
        assert_eq!(r.item, Some(ValueMeta(6 + (30 << 4))));
        assert_eq!(r.group, 0);

        let r = iter.next().unwrap();
        assert_eq!(r.item, Some(ValueMeta(6 + (10 << 4))));
        assert_eq!(r.group, 30);

        let r = iter.next().unwrap();
        assert_eq!(r.item, Some(ValueMeta(3)));
        assert_eq!(r.group, 40);

        let mut iter = col.iter().with_group();
        iter.advance_by(3);

        let r = iter.next().unwrap();
        assert_eq!(r.item, Some(ValueMeta(3)));
        assert_eq!(r.group, 40);

        let mut iter = col.iter_range(3..5).with_group();

        let r = iter.next().unwrap();
        assert_eq!(r.item, Some(ValueMeta(3)));
        assert_eq!(r.group, 40);
    }
}