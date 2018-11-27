#![allow(renamed_and_removed_lints)]
#![allow(non_snake_case)]

//extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate serde_json;
extern crate serde;

#[macro_use]
extern crate tera;
#[cfg(feature = "filesystem")]
extern crate walkdir;

#[cfg(feature = "filesystem")]
extern crate dirs;

#[macro_use]
extern crate log;

extern crate reqwest;

extern crate regex;

extern crate semver;
extern crate base64;

#[macro_use] extern crate failure;

pub use failure::Error; //Fail
pub type Result<T> = std::result::Result<T, Error>;

// Mutually exclusive backing on hold atm..
// currently breaks Default for ConfigType and ManifestType
//#[macro_use]
//extern crate static_assertions;
//assert_cfg!(all(not(all(feature = "filesystem", feature = "crd")),
//                any(    feature = "filesystem", feature = "crd")),
//            "Must exclusively use Filesystem or CRDs as the backing");


/// Config with regional data
pub mod region;
pub use region::{Region, VaultConfig, VersionScheme, KongConfig};
/// Master config with cross-region data
pub mod config;
pub use config::{Config, Cluster, Team, ManifestDefaults};


/// Structs for the manifest
pub mod structs;

pub mod manifest;
pub use manifest::Manifest;

/// Crd wrappers
mod crds;
pub use crds::{Crd, CrdList};

/// Internal classifications and states
mod states;
pub use states::{ConfigType};

/// File backing
#[cfg(feature = "filesystem")]
mod filebacked;

// Merge behaviour for manifests
mod merge;

/// Computational helpers
pub mod math;


/// A renderer of `tera` templates (jinja style)
///
/// Used for small app configs that are inlined in the completed manifests.
pub mod template;

//pub mod product;
//pub use product::Product;

/// A Hashicorp Vault HTTP client using `reqwest`
pub mod vault;
pub use vault::Vault;
