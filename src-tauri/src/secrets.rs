use std::fmt;

/// An opaque reference persisted in Cookbench configuration. Secret values are
/// never serialized, logged, or passed to the frontend.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SecretReference {
    service: String,
    account: String,
}

impl SecretReference {
    pub const MAX_COMPONENT_BYTES: usize = 128;

    pub fn new(
        service: impl Into<String>,
        account: impl Into<String>,
    ) -> Result<Self, SecretError> {
        let service = service.into();
        let account = account.into();
        if service.is_empty()
            || account.is_empty()
            || service.len() > Self::MAX_COMPONENT_BYTES
            || account.len() > Self::MAX_COMPONENT_BYTES
        {
            return Err(SecretError::InvalidReference);
        }
        Ok(Self { service, account })
    }

    pub fn service(&self) -> &str {
        &self.service
    }
    pub fn account(&self) -> &str {
        &self.account
    }

    pub fn redacted(&self) -> String {
        format!("secret://{}/{}", self.service, self.account)
    }
}

/// Implement with the native OS credential store (Keychain, Credential Manager,
/// or libsecret). Callers receive values only at send time.
pub trait SecretStore: Send + Sync {
    fn get(&self, reference: &SecretReference) -> Result<String, SecretError>;
    fn set(&self, reference: &SecretReference, value: &str) -> Result<(), SecretError>;
    fn delete(&self, reference: &SecretReference) -> Result<(), SecretError>;
}

/// Cross-platform OS credential-store implementation backed by Keychain on
/// macOS, Credential Manager on Windows, and Secret Service on Linux.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeSecretStore;

impl NativeSecretStore {
    fn entry(reference: &SecretReference) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(reference.service(), reference.account())
            .map_err(|_| SecretError::Unavailable)
    }
}

impl SecretStore for NativeSecretStore {
    fn get(&self, reference: &SecretReference) -> Result<String, SecretError> {
        Self::entry(reference)?
            .get_password()
            .map_err(|error| match error {
                keyring::Error::NoEntry => SecretError::NotFound,
                _ => SecretError::StorageFailure,
            })
    }

    fn set(&self, reference: &SecretReference, value: &str) -> Result<(), SecretError> {
        if value.is_empty() {
            return Err(SecretError::StorageFailure);
        }
        Self::entry(reference)?
            .set_password(value)
            .map_err(|_| SecretError::StorageFailure)
    }

    fn delete(&self, reference: &SecretReference) -> Result<(), SecretError> {
        Self::entry(reference)?
            .delete_credential()
            .map_err(|error| match error {
                keyring::Error::NoEntry => SecretError::NotFound,
                _ => SecretError::StorageFailure,
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SecretError {
    InvalidReference,
    Unavailable,
    NotFound,
    StorageFailure,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReference => f.write_str("invalid secret reference"),
            Self::Unavailable => f.write_str("OS credential store unavailable"),
            Self::NotFound => f.write_str("secret reference not found"),
            Self::StorageFailure => f.write_str("OS credential store failed"),
        }
    }
}
impl std::error::Error for SecretError {}
