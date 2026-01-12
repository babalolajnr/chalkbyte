//! Strongly-typed value types with validation for domain primitives.
//!
//! This module provides newtype wrappers for common validated values like
//! email addresses and phone numbers, ensuring they are always valid when used.
//!
//! # Example
//!
//! ```ignore
//! use chalkbyte_models::value_types::{Email, PhoneNumber};
//!
//! // Parse and validate
//! let email: Email = "user@example.com".parse().unwrap();
//! let phone: PhoneNumber = "+1234567890".parse().unwrap();
//!
//! // Use as string
//! println!("Email: {}", email);
//! println!("Phone: {}", phone.as_str());
//! ```

use serde::{Deserialize, Serialize};
use sqlx::{
    Database, Decode, Encode, Type,
    postgres::{PgHasArrayType, PgTypeInfo},
};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;
use validator::ValidateEmail;

/// Error type for value type parsing failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueTypeError {
    /// The email address is invalid.
    InvalidEmail(String),
    /// The phone number is invalid.
    InvalidPhoneNumber(String),
}

impl std::error::Error for ValueTypeError {}

impl fmt::Display for ValueTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEmail(msg) => write!(f, "Invalid email: {}", msg),
            Self::InvalidPhoneNumber(msg) => write!(f, "Invalid phone number: {}", msg),
        }
    }
}

// ============================================================================
// Email
// ============================================================================

/// A validated email address.
///
/// This type guarantees that the contained string is a valid email address
/// according to the validator crate's email validation rules.
///
/// # Example
///
/// ```ignore
/// use chalkbyte_models::value_types::Email;
///
/// let email: Email = "user@example.com".parse().unwrap();
/// assert_eq!(email.as_str(), "user@example.com");
///
/// // Invalid emails fail to parse
/// assert!("not-an-email".parse::<Email>().is_err());
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[schema(value_type = String, format = "email", example = "user@example.com")]
pub struct Email(String);

impl Email {
    /// Create a new Email from a string, validating it.
    ///
    /// Returns `Err` if the email is invalid.
    pub fn new(email: impl Into<String>) -> Result<Self, ValueTypeError> {
        let email = email.into();
        Self::validate(&email)?;
        Ok(Self(email))
    }

    /// Create an Email without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the email is valid. This is intended for use
    /// when loading from a trusted source (e.g., database) where validation
    /// was already performed.
    #[inline]
    pub fn new_unchecked(email: impl Into<String>) -> Self {
        Self(email.into())
    }

    /// Get the email as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume self and return the inner String.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Get the local part (before @) of the email.
    pub fn local_part(&self) -> &str {
        self.0.split('@').next().unwrap_or("")
    }

    /// Get the domain part (after @) of the email.
    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap_or("")
    }

    /// Validate an email string.
    fn validate(email: &str) -> Result<(), ValueTypeError> {
        if email.is_empty() {
            return Err(ValueTypeError::InvalidEmail("email cannot be empty".into()));
        }

        if !email.validate_email() {
            return Err(ValueTypeError::InvalidEmail(format!(
                "'{}' is not a valid email address",
                email
            )));
        }

        Ok(())
    }
}

impl fmt::Debug for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Email({})", self.0)
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Email {
    type Err = ValueTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for Email {
    type Error = ValueTypeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Email {
    type Error = ValueTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl AsRef<str> for Email {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<Email> for String {
    fn from(email: Email) -> String {
        email.0
    }
}

impl PartialEq<str> for Email {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<String> for Email {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

// SQLx Type implementation for Postgres
impl Type<sqlx::Postgres> for Email {
    fn type_info() -> PgTypeInfo {
        <String as Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as Type<sqlx::Postgres>>::compatible(ty)
    }
}

// SQLx Encode implementation
impl<'q> Encode<'q, sqlx::Postgres> for Email {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as Encode<'q, sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

// SQLx Decode implementation
impl<'r> Decode<'r, sqlx::Postgres> for Email {
    fn decode(
        value: <sqlx::Postgres as Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as Decode<'r, sqlx::Postgres>>::decode(value)?;
        // Trust database values - they should already be validated
        Ok(Self::new_unchecked(s))
    }
}

// SQLx array type support for Postgres
impl PgHasArrayType for Email {
    fn array_type_info() -> PgTypeInfo {
        <String as PgHasArrayType>::array_type_info()
    }
}

// Serde Deserialize with validation
impl<'de> Deserialize<'de> for Email {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

// ============================================================================
// PhoneNumber
// ============================================================================

/// A validated phone number.
///
/// This type validates phone numbers using a simple but effective regex pattern
/// that accepts international formats. The validation ensures:
/// - Optional leading `+` for country code
/// - Only digits, spaces, dashes, and parentheses allowed
/// - Minimum 7 digits, maximum 15 digits (E.164 standard)
///
/// # Example
///
/// ```ignore
/// use chalkbyte_models::value_types::PhoneNumber;
///
/// let phone: PhoneNumber = "+1 (555) 123-4567".parse().unwrap();
/// assert_eq!(phone.digits_only(), "15551234567");
///
/// // Invalid phones fail to parse
/// assert!("abc".parse::<PhoneNumber>().is_err());
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[schema(value_type = String, example = "+1234567890")]
pub struct PhoneNumber(String);

impl PhoneNumber {
    /// Minimum number of digits in a valid phone number.
    const MIN_DIGITS: usize = 7;
    /// Maximum number of digits in a valid phone number (E.164 standard).
    const MAX_DIGITS: usize = 15;

    /// Create a new PhoneNumber from a string, validating it.
    ///
    /// Returns `Err` if the phone number is invalid.
    pub fn new(phone: impl Into<String>) -> Result<Self, ValueTypeError> {
        let phone = phone.into();
        Self::validate(&phone)?;
        Ok(Self(phone))
    }

    /// Create a PhoneNumber without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the phone number is valid. This is intended for use
    /// when loading from a trusted source (e.g., database) where validation
    /// was already performed.
    #[inline]
    pub fn new_unchecked(phone: impl Into<String>) -> Self {
        Self(phone.into())
    }

    /// Get the phone number as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume self and return the inner String.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Get only the digits from the phone number.
    ///
    /// This strips all formatting characters and returns just the numeric digits.
    pub fn digits_only(&self) -> String {
        self.0.chars().filter(|c| c.is_ascii_digit()).collect()
    }

    /// Normalize the phone number to E.164 format (digits only with leading +).
    ///
    /// Note: This is a simple normalization that assumes the input already
    /// contains the country code if it starts with +.
    pub fn to_e164(&self) -> String {
        let digits = self.digits_only();
        if self.0.starts_with('+') {
            format!("+{}", digits)
        } else {
            digits
        }
    }

    /// Validate a phone number string.
    fn validate(phone: &str) -> Result<(), ValueTypeError> {
        if phone.is_empty() {
            return Err(ValueTypeError::InvalidPhoneNumber(
                "phone number cannot be empty".into(),
            ));
        }

        // Check for valid characters: digits, +, -, (), spaces
        let valid_chars = phone.chars().all(|c| {
            c.is_ascii_digit() || c == '+' || c == '-' || c == '(' || c == ')' || c == ' '
        });

        if !valid_chars {
            return Err(ValueTypeError::InvalidPhoneNumber(format!(
                "'{}' contains invalid characters",
                phone
            )));
        }

        // + can only appear at the start
        if phone.chars().skip(1).any(|c| c == '+') {
            return Err(ValueTypeError::InvalidPhoneNumber(
                "+ can only appear at the start".into(),
            ));
        }

        // Count digits
        let digit_count = phone.chars().filter(|c| c.is_ascii_digit()).count();

        if digit_count < Self::MIN_DIGITS {
            return Err(ValueTypeError::InvalidPhoneNumber(format!(
                "phone number must have at least {} digits, got {}",
                Self::MIN_DIGITS,
                digit_count
            )));
        }

        if digit_count > Self::MAX_DIGITS {
            return Err(ValueTypeError::InvalidPhoneNumber(format!(
                "phone number must have at most {} digits, got {}",
                Self::MAX_DIGITS,
                digit_count
            )));
        }

        Ok(())
    }
}

impl fmt::Debug for PhoneNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhoneNumber({})", self.0)
    }
}

impl fmt::Display for PhoneNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PhoneNumber {
    type Err = ValueTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for PhoneNumber {
    type Error = ValueTypeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for PhoneNumber {
    type Error = ValueTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl AsRef<str> for PhoneNumber {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<PhoneNumber> for String {
    fn from(phone: PhoneNumber) -> String {
        phone.0
    }
}

impl PartialEq<str> for PhoneNumber {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<String> for PhoneNumber {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

// SQLx Type implementation for Postgres
impl Type<sqlx::Postgres> for PhoneNumber {
    fn type_info() -> PgTypeInfo {
        <String as Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as Type<sqlx::Postgres>>::compatible(ty)
    }
}

// SQLx Encode implementation
impl<'q> Encode<'q, sqlx::Postgres> for PhoneNumber {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as Encode<'q, sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

// SQLx Decode implementation
impl<'r> Decode<'r, sqlx::Postgres> for PhoneNumber {
    fn decode(
        value: <sqlx::Postgres as Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as Decode<'r, sqlx::Postgres>>::decode(value)?;
        // Trust database values - they should already be validated
        Ok(Self::new_unchecked(s))
    }
}

// SQLx array type support for Postgres
impl PgHasArrayType for PhoneNumber {
    fn array_type_info() -> PgTypeInfo {
        <String as PgHasArrayType>::array_type_info()
    }
}

// Serde Deserialize with validation
impl<'de> Deserialize<'de> for PhoneNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Email tests
    mod email_tests {
        use super::*;

        #[test]
        fn test_valid_email() {
            assert!(Email::new("user@example.com").is_ok());
            assert!(Email::new("test.user@example.co.uk").is_ok());
            assert!(Email::new("user+tag@example.com").is_ok());
            assert!(Email::new("user123@test.org").is_ok());
        }

        #[test]
        fn test_invalid_email() {
            assert!(Email::new("").is_err());
            assert!(Email::new("not-an-email").is_err());
            assert!(Email::new("@example.com").is_err());
            assert!(Email::new("user@").is_err());
            assert!(Email::new("user@@example.com").is_err());
        }

        #[test]
        fn test_email_parts() {
            let email = Email::new("user@example.com").unwrap();
            assert_eq!(email.local_part(), "user");
            assert_eq!(email.domain(), "example.com");
        }

        #[test]
        fn test_email_display() {
            let email = Email::new("user@example.com").unwrap();
            assert_eq!(format!("{}", email), "user@example.com");
        }

        #[test]
        fn test_email_debug() {
            let email = Email::new("user@example.com").unwrap();
            assert_eq!(format!("{:?}", email), "Email(user@example.com)");
        }

        #[test]
        fn test_email_parse() {
            let email: Email = "user@example.com".parse().unwrap();
            assert_eq!(email.as_str(), "user@example.com");
        }

        #[test]
        fn test_email_equality() {
            let email1 = Email::new("user@example.com").unwrap();
            let email2 = Email::new("user@example.com").unwrap();
            assert_eq!(email1, email2);
            assert_eq!(email1.as_str(), "user@example.com");
            assert_eq!(email1.as_str(), "user@example.com".to_string());
        }

        #[test]
        fn test_email_serialize() {
            let email = Email::new("user@example.com").unwrap();
            let json = serde_json::to_string(&email).unwrap();
            assert_eq!(json, r#""user@example.com""#);
        }

        #[test]
        fn test_email_deserialize_valid() {
            let json = r#""user@example.com""#;
            let email: Email = serde_json::from_str(json).unwrap();
            assert_eq!(email.as_str(), "user@example.com");
        }

        #[test]
        fn test_email_deserialize_invalid() {
            let json = r#""not-an-email""#;
            let result: Result<Email, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }

        #[test]
        fn test_email_hash() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(Email::new("user1@example.com").unwrap());
            set.insert(Email::new("user2@example.com").unwrap());
            assert_eq!(set.len(), 2);
            set.insert(Email::new("user1@example.com").unwrap());
            assert_eq!(set.len(), 2);
        }

        #[test]
        fn test_email_into_string() {
            let email = Email::new("user@example.com").unwrap();
            let s: String = email.into();
            assert_eq!(s, "user@example.com");
        }

        #[test]
        fn test_email_try_from() {
            let email = Email::try_from("user@example.com").unwrap();
            assert_eq!(email.as_str(), "user@example.com");

            let email = Email::try_from("user@example.com".to_string()).unwrap();
            assert_eq!(email.as_str(), "user@example.com");
        }
    }

    // PhoneNumber tests
    mod phone_tests {
        use super::*;

        #[test]
        fn test_valid_phone() {
            assert!(PhoneNumber::new("+1234567890").is_ok());
            assert!(PhoneNumber::new("1234567890").is_ok());
            assert!(PhoneNumber::new("+1 (555) 123-4567").is_ok());
            assert!(PhoneNumber::new("555-123-4567").is_ok());
            assert!(PhoneNumber::new("(555) 123-4567").is_ok());
        }

        #[test]
        fn test_invalid_phone_empty() {
            assert!(PhoneNumber::new("").is_err());
        }

        #[test]
        fn test_invalid_phone_characters() {
            assert!(PhoneNumber::new("abc1234567").is_err());
            assert!(PhoneNumber::new("123.456.7890").is_err());
        }

        #[test]
        fn test_invalid_phone_plus_position() {
            assert!(PhoneNumber::new("123+4567890").is_err());
        }

        #[test]
        fn test_invalid_phone_too_few_digits() {
            assert!(PhoneNumber::new("123456").is_err());
        }

        #[test]
        fn test_invalid_phone_too_many_digits() {
            assert!(PhoneNumber::new("1234567890123456").is_err());
        }

        #[test]
        fn test_phone_digits_only() {
            let phone = PhoneNumber::new("+1 (555) 123-4567").unwrap();
            assert_eq!(phone.digits_only(), "15551234567");
        }

        #[test]
        fn test_phone_to_e164() {
            let phone = PhoneNumber::new("+1 (555) 123-4567").unwrap();
            assert_eq!(phone.to_e164(), "+15551234567");

            let phone = PhoneNumber::new("555-123-4567").unwrap();
            assert_eq!(phone.to_e164(), "5551234567");
        }

        #[test]
        fn test_phone_display() {
            let phone = PhoneNumber::new("+1234567890").unwrap();
            assert_eq!(format!("{}", phone), "+1234567890");
        }

        #[test]
        fn test_phone_debug() {
            let phone = PhoneNumber::new("+1234567890").unwrap();
            assert_eq!(format!("{:?}", phone), "PhoneNumber(+1234567890)");
        }

        #[test]
        fn test_phone_parse() {
            let phone: PhoneNumber = "+1234567890".parse().unwrap();
            assert_eq!(phone.as_str(), "+1234567890");
        }

        #[test]
        fn test_phone_equality() {
            let phone1 = PhoneNumber::new("+1234567890").unwrap();
            let phone2 = PhoneNumber::new("+1234567890").unwrap();
            assert_eq!(phone1, phone2);
            assert_eq!(phone1.as_str(), "+1234567890");
            assert_eq!(phone1.as_str(), "+1234567890".to_string());
        }

        #[test]
        fn test_phone_serialize() {
            let phone = PhoneNumber::new("+1234567890").unwrap();
            let json = serde_json::to_string(&phone).unwrap();
            assert_eq!(json, r#""+1234567890""#);
        }

        #[test]
        fn test_phone_deserialize_valid() {
            let json = r#""+1234567890""#;
            let phone: PhoneNumber = serde_json::from_str(json).unwrap();
            assert_eq!(phone.as_str(), "+1234567890");
        }

        #[test]
        fn test_phone_deserialize_invalid() {
            let json = r#""abc""#;
            let result: Result<PhoneNumber, _> = serde_json::from_str(json);
            assert!(result.is_err());
        }

        #[test]
        fn test_phone_hash() {
            use std::collections::HashSet;
            let mut set = HashSet::new();
            set.insert(PhoneNumber::new("+1234567890").unwrap());
            set.insert(PhoneNumber::new("+0987654321").unwrap());
            assert_eq!(set.len(), 2);
            set.insert(PhoneNumber::new("+1234567890").unwrap());
            assert_eq!(set.len(), 2);
        }

        #[test]
        fn test_phone_into_string() {
            let phone = PhoneNumber::new("+1234567890").unwrap();
            let s: String = phone.into();
            assert_eq!(s, "+1234567890");
        }

        #[test]
        fn test_phone_try_from() {
            let phone = PhoneNumber::try_from("+1234567890").unwrap();
            assert_eq!(phone.as_str(), "+1234567890");

            let phone = PhoneNumber::try_from("+1234567890".to_string()).unwrap();
            assert_eq!(phone.as_str(), "+1234567890");
        }
    }

    // Error tests
    mod error_tests {
        use super::*;

        #[test]
        fn test_error_display() {
            let err = ValueTypeError::InvalidEmail("test".into());
            assert_eq!(format!("{}", err), "Invalid email: test");

            let err = ValueTypeError::InvalidPhoneNumber("test".into());
            assert_eq!(format!("{}", err), "Invalid phone number: test");
        }

        #[test]
        fn test_error_is_std_error() {
            fn assert_error<E: std::error::Error>() {}
            assert_error::<ValueTypeError>();
        }
    }
}
