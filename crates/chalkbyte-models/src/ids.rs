//! Strongly-typed ID newtypes for domain entities.
//!
//! This module provides newtype wrappers around `Uuid` for each entity type,
//! preventing accidental misuse of IDs (e.g., passing a `SchoolId` where a `UserId` is expected).
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_models::ids::{UserId, SchoolId};
//!
//! fn get_user(id: UserId) { /* ... */ }
//! fn get_school(id: SchoolId) { /* ... */ }
//!
//! let user_id = UserId::new();
//! let school_id = SchoolId::new();
//!
//! get_user(user_id);    // OK
//! // get_user(school_id); // Compile error! Type mismatch.
//! ```

use serde::{Deserialize, Serialize};
use sqlx::{
    Database, Decode, Encode, Type,
    postgres::{PgHasArrayType, PgTypeInfo},
};
use std::fmt;
use utoipa::ToSchema;
use uuid::Uuid;

/// Macro to define a strongly-typed ID newtype.
///
/// This macro generates a newtype wrapper around `Uuid` with all necessary
/// trait implementations for database operations, serialization, and API documentation.
macro_rules! define_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, ToSchema)]
        #[schema(value_type = String, format = "uuid")]
        pub struct $name(pub Uuid);

        impl $name {
            /// Create a new random ID.
            #[inline]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create an ID from an existing UUID.
            #[inline]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Create an ID from a u128 value (useful for constants).
            #[inline]
            pub const fn from_u128(v: u128) -> Self {
                Self(Uuid::from_u128(v))
            }

            /// Get the inner UUID value.
            #[inline]
            pub const fn into_inner(self) -> Uuid {
                self.0
            }

            /// Get a reference to the inner UUID.
            #[inline]
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Create a nil (all zeros) ID.
            #[inline]
            pub const fn nil() -> Self {
                Self(Uuid::nil())
            }

            /// Check if this is a nil ID.
            #[inline]
            pub fn is_nil(&self) -> bool {
                self.0.is_nil()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            #[inline]
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            #[inline]
            fn from(id: $name) -> Uuid {
                id.0
            }
        }

        impl AsRef<Uuid> for $name {
            #[inline]
            fn as_ref(&self) -> &Uuid {
                &self.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        // SQLx Type implementation for Postgres
        impl Type<sqlx::Postgres> for $name {
            fn type_info() -> PgTypeInfo {
                <Uuid as Type<sqlx::Postgres>>::type_info()
            }

            fn compatible(ty: &PgTypeInfo) -> bool {
                <Uuid as Type<sqlx::Postgres>>::compatible(ty)
            }
        }

        // SQLx Encode implementation
        impl<'q> Encode<'q, sqlx::Postgres> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <sqlx::Postgres as Database>::ArgumentBuffer<'q>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <Uuid as Encode<'q, sqlx::Postgres>>::encode_by_ref(&self.0, buf)
            }
        }

        // SQLx Decode implementation
        impl<'r> Decode<'r, sqlx::Postgres> for $name {
            fn decode(
                value: <sqlx::Postgres as Database>::ValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                <Uuid as Decode<'r, sqlx::Postgres>>::decode(value).map(Self)
            }
        }

        // SQLx array type support for Postgres
        impl PgHasArrayType for $name {
            fn array_type_info() -> PgTypeInfo {
                <Uuid as PgHasArrayType>::array_type_info()
            }
        }

        // Serde Deserialize - manual impl for transparent UUID deserialization
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Uuid::deserialize(deserializer).map(Self)
            }
        }
    };
}

// Define all entity ID types
define_id!(
    /// Strongly-typed ID for User entities.
    UserId
);

define_id!(
    /// Strongly-typed ID for School entities.
    SchoolId
);

define_id!(
    /// Strongly-typed ID for Level entities.
    LevelId
);

define_id!(
    /// Strongly-typed ID for Branch entities.
    BranchId
);

define_id!(
    /// Strongly-typed ID for Role entities.
    RoleId
);

define_id!(
    /// Strongly-typed ID for Permission entities.
    PermissionId
);

define_id!(
    /// Strongly-typed ID for RolePermission junction entities.
    RolePermissionId
);

define_id!(
    /// Strongly-typed ID for UserRole junction entities.
    UserRoleId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id = UserId::new();
        assert!(!id.is_nil());
    }

    #[test]
    fn test_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = UserId::from_uuid(uuid);
        assert_eq!(id.into_inner(), uuid);
    }

    #[test]
    fn test_id_from_u128() {
        let id = RoleId::from_u128(0x00000000_0000_0000_0000_000000000001);
        assert_eq!(
            id.into_inner(),
            Uuid::from_u128(0x00000000_0000_0000_0000_000000000001)
        );
    }

    #[test]
    fn test_id_nil() {
        let id = SchoolId::nil();
        assert!(id.is_nil());
    }

    #[test]
    fn test_id_equality() {
        let uuid = Uuid::new_v4();
        let id1 = LevelId::from_uuid(uuid);
        let id2 = LevelId::from_uuid(uuid);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_id_inequality_same_uuid_different_types() {
        // This test ensures type safety - same UUID, different types
        // These should NOT be equal at compile time (different types)
        let uuid = Uuid::new_v4();
        let _user_id = UserId::from_uuid(uuid);
        let _school_id = SchoolId::from_uuid(uuid);
        // If this compiled: assert_ne!(user_id, school_id);
        // It won't compile because they're different types - which is the point!
    }

    #[test]
    fn test_id_debug() {
        let id = UserId::from_u128(0x12345678_1234_1234_1234_123456789abc);
        let debug = format!("{:?}", id);
        assert!(debug.starts_with("UserId("));
        assert!(debug.contains("12345678-1234-1234-1234-123456789abc"));
    }

    #[test]
    fn test_id_display() {
        let uuid = Uuid::from_u128(0x12345678_1234_1234_1234_123456789abc);
        let id = BranchId::from_uuid(uuid);
        assert_eq!(format!("{}", id), "12345678-1234-1234-1234-123456789abc");
    }

    #[test]
    fn test_id_from_str() {
        let id: UserId = "12345678-1234-1234-1234-123456789abc".parse().unwrap();
        assert_eq!(
            id.into_inner(),
            Uuid::from_u128(0x12345678_1234_1234_1234_123456789abc)
        );
    }

    #[test]
    fn test_id_from_str_invalid() {
        let result: Result<UserId, _> = "invalid-uuid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_id_serialize() {
        let id = PermissionId::from_u128(0x12345678_1234_1234_1234_123456789abc);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""12345678-1234-1234-1234-123456789abc""#);
    }

    #[test]
    fn test_id_deserialize() {
        let json = r#""12345678-1234-1234-1234-123456789abc""#;
        let id: RoleId = serde_json::from_str(json).unwrap();
        assert_eq!(
            id.into_inner(),
            Uuid::from_u128(0x12345678_1234_1234_1234_123456789abc)
        );
    }

    #[test]
    fn test_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        let id1 = UserId::new();
        let id2 = UserId::new();
        set.insert(id1);
        set.insert(id2);
        assert_eq!(set.len(), 2);
        set.insert(id1); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_id_clone_copy() {
        let id = SchoolId::new();
        let cloned = id.clone();
        let copied = id; // Copy
        assert_eq!(id, cloned);
        assert_eq!(id, copied);
    }

    #[test]
    fn test_id_as_ref() {
        let uuid = Uuid::new_v4();
        let id = LevelId::from_uuid(uuid);
        assert_eq!(id.as_ref(), &uuid);
    }

    #[test]
    fn test_id_conversion_roundtrip() {
        let original_uuid = Uuid::new_v4();
        let id: BranchId = original_uuid.into();
        let recovered_uuid: Uuid = id.into();
        assert_eq!(original_uuid, recovered_uuid);
    }
}
