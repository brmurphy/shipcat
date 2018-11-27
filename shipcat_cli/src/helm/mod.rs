/// Allow normal error handling from structs

// New failure error type
#[derive(Debug)]
struct HError {
    inner: Context<HErrKind>,
}
// its associated enum
#[derive(Clone, Eq, PartialEq, Debug, Fail)]
enum HErrKind {
    #[fail(display = "{} has no version in manifest and is not installed yes", _0)]
    MissingRollingVersion(String),

    #[fail(display = "manifest key '{}' was not propagated internally - bug!", _0)]
    ManifestFailure(String),

    #[fail(display = "Helm upgrade of '{}' failed", _0)]
    HelmUpgradeFailure(String),

    #[fail(display = "{} upgrade timed out waiting{}s for deployment(s) to come online", _0, _1)]
    UpgradeTimeout(String, u32),
}
use failure::{Error, Fail, Context, Backtrace, ResultExt};
use std::fmt::{self, Display};

// boilerplate error wrapping (might go away)
impl Fail for HError {
    fn cause(&self) -> Option<&Fail> { self.inner.cause() }
    fn backtrace(&self) -> Option<&Backtrace> { self.inner.backtrace() }
}
impl Display for HError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
impl From<HErrKind> for HError {
    fn from(kind: HErrKind) -> HError {
        HError { inner: Context::new(kind) }
    }
}
impl From<Context<HErrKind>> for HError {
    fn from(inner: Context<HErrKind>) -> HError {
        HError { inner: inner }
    }
}
pub type Result<T> = std::result::Result<T, Error>;




/// Verify trait gets the Config
pub use super::{Config, Region, VersionScheme};
/// Need basic manifest handling
pub use super::Manifest;

/// For slack hookback
pub use super::structs::Metadata;

// allow using some slack and kube stuff
pub use super::slack;
pub use super::grafana;
pub use super::kube;

/// Parallel helm invokers
pub mod parallel;

/// Direct helm invokers (used by abstractions)
pub mod direct;
// Re-exports for main
pub use self::direct::{history, template, values, status};

/// Helm related helpers
pub mod helpers;
// Commonly used helper
pub use self::helpers::infer_fallback_version;

pub use self::direct::{UpgradeMode, UpgradeData};
