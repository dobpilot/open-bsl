//! Криптографические примитивы BSL, необходимые HTTP-клиенту.

use std::cell::RefCell;
use std::fmt;

use bsl_rt::{
    Arity, BslValue, CallContext, ConstructorCode, ConstructorDescriptor, EnumValue,
    LibraryDescriptor, MethodDescriptor, ObjectContextNeed, ObjectProtocol, PropertyDescriptor,
    RtError, RtResult, TypeDescriptor,
};
use digest::Digest;

/// Идентификатор компонента в заголовке байт-кода.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
/// Версия компонента в заголовке байт-кода.
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub static DATA_HASHING_TYPE: TypeDescriptor = TypeDescriptor {
    package: PACKAGE_NAME,
    name: "ХешированиеДанных",
    type_display: "DataHashing",
    type_names: &["DataHashing"],
};

#[derive(Clone)]
enum Hasher {
    Md5(md5::Md5),
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
}

impl fmt::Debug for Hasher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Md5(_) => "MD5",
            Self::Sha1(_) => "SHA-1",
            Self::Sha256(_) => "SHA-256",
        })
    }
}

impl Hasher {
    fn new(algorithm: EnumValue) -> RtResult<Self> {
        match algorithm {
            EnumValue::HashMd5 => Ok(Self::Md5(md5::Md5::new())),
            EnumValue::HashSha1 => Ok(Self::Sha1(sha1::Sha1::new())),
            EnumValue::HashSha256 => Ok(Self::Sha256(sha2::Sha256::new())),
            _ => Err(RtError::TypeError {
                expected: "ХешФункция",
                op: "Новый ХешированиеДанных",
            }),
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Md5(hasher) => hasher.update(bytes),
            Self::Sha1(hasher) => hasher.update(bytes),
            Self::Sha256(hasher) => hasher.update(bytes),
        }
    }

    fn snapshot(&self) -> Vec<u8> {
        match self {
            Self::Md5(hasher) => hasher.clone().finalize().to_vec(),
            Self::Sha1(hasher) => hasher.clone().finalize().to_vec(),
            Self::Sha256(hasher) => hasher.clone().finalize().to_vec(),
        }
    }
}

#[derive(Debug)]
struct DataHashingObject {
    hasher: RefCell<Hasher>,
}

impl ObjectProtocol for DataHashingObject {
    fn type_descriptor(&self) -> &'static TypeDescriptor {
        &DATA_HASHING_TYPE
    }

    fn property_table(&self) -> &'static [PropertyDescriptor] {
        DATA_HASHING_PROPERTIES
    }

    fn method_table(&self) -> &'static [MethodDescriptor] {
        DATA_HASHING_METHODS
    }
}

fn data_hashing(receiver: &dyn ObjectProtocol) -> RtResult<&DataHashingObject> {
    receiver
        .downcast_ref::<DataHashingObject>()
        .ok_or(RtError::NotAnObject)
}

fn add(
    receiver: &dyn ObjectProtocol,
    arguments: &[BslValue],
    _context: &mut CallContext<'_>,
) -> RtResult<BslValue> {
    let bytes = arguments[0].binary_data_bytes().ok_or(RtError::TypeError {
        expected: "ДвоичныеДанные",
        op: "ХешированиеДанных.Добавить",
    })?;
    data_hashing(receiver)?.hasher.borrow_mut().update(bytes);
    Ok(BslValue::Undefined)
}

fn hash_sum(receiver: &dyn ObjectProtocol, _context: &mut CallContext<'_>) -> RtResult<BslValue> {
    let bytes = data_hashing(receiver)?.hasher.borrow().snapshot();
    Ok(BslValue::binary_data_of(bytes))
}

const DATA_HASHING_METHODS: &[MethodDescriptor] = &[MethodDescriptor::new(
    &["Добавить", "Add"],
    Arity::exact(1),
    add,
)];

const DATA_HASHING_PROPERTIES: &[PropertyDescriptor] = &[PropertyDescriptor {
    names: &["ХешСумма", "HashSum"],
    get: hash_sum,
    set: None,
}];

const API_MEMBERS: &[bsl_rt::ObjectMembersDescriptor] =
    &[bsl_rt::ObjectMembersDescriptor::new(&DATA_HASHING_TYPE)
        .with_properties(DATA_HASHING_PROPERTIES)
        .with_methods(DATA_HASHING_METHODS)];

const OBJECT_MEMBER_GROUPS: &[&[bsl_rt::ObjectMembersDescriptor]] = &[API_MEMBERS];

fn construct_data_hashing(
    _context: &mut CallContext<'_>,
    arguments: &[BslValue],
) -> RtResult<BslValue> {
    let BslValue::Enum(algorithm) = arguments[0] else {
        return Err(RtError::TypeError {
            expected: "ХешФункция",
            op: "Новый ХешированиеДанных",
        });
    };
    Ok(BslValue::new_object(DataHashingObject {
        hasher: RefCell::new(Hasher::new(algorithm)?),
    }))
}

const CONSTRUCTORS: &[ConstructorDescriptor] = &[ConstructorDescriptor {
    code: ConstructorCode::new(1),
    names: &["ХешированиеДанных", "DataHashing"],
    arity: Arity::exact(1),
    call: construct_data_hashing,
}];

const TYPES: &[&TypeDescriptor] = &[&DATA_HASHING_TYPE];

/// Дескриптор компонента криптографии.
#[must_use]
pub const fn library() -> LibraryDescriptor {
    LibraryDescriptor::new(PACKAGE_NAME, PACKAGE_VERSION, ObjectContextNeed::Reduced)
        .with_constructors(CONSTRUCTORS)
        .with_types(TYPES)
        .with_object_member_groups(OBJECT_MEMBER_GROUPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02X}")).collect()
    }

    #[test]
    fn official_abc_vectors_match_all_supported_algorithms() {
        for (algorithm, expected) in [
            (EnumValue::HashMd5, "900150983CD24FB0D6963F7D28E17F72"),
            (
                EnumValue::HashSha1,
                "A9993E364706816ABA3E25717850C26C9CD0D89D",
            ),
            (
                EnumValue::HashSha256,
                "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
            ),
        ] {
            let mut hasher = Hasher::new(algorithm).unwrap();
            hasher.update(b"abc");
            assert_eq!(hex(&hasher.snapshot()), expected);
        }
    }

    #[test]
    fn snapshot_does_not_consume_the_hasher() {
        let mut hasher = Hasher::new(EnumValue::HashSha256).unwrap();
        hasher.update(b"a");
        let first = hasher.snapshot();
        assert_eq!(hasher.snapshot(), first);
        hasher.update(b"bc");
        assert_eq!(
            hex(&hasher.snapshot()),
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        );
    }
}
