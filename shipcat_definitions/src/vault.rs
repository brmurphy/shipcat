use std::collections::BTreeMap;
use std::env;
use std::io::Read;

use region::{VaultConfig};

// All main errors that can happen from vault
#[derive(Debug)]
struct VaultError {
    inner: Context<VErrKind>,
}

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
enum VErrKind {
    #[fail(display = "secret '{}' not have the 'value' key", _0)]
    InvalidSecretForm(String),

    #[fail(display = "secret '{}' could not be reached or accessed", _0)]
    SecretNotAccessible(String),

    #[fail(display = "VAULT_ADDR not specified")]
    MissingAddr,

    #[fail(display = "VAULT_TOKEN not specified")]
    MissingToken,

    #[fail(display = "Unexpected HTTP status {} from {}", _0, _1)]
    UnexpectedHttpStatus(reqwest::StatusCode, String),

    #[fail(display = "could not access URL '{}'", _0)]
    Url(reqwest::Url),
}
use failure::{Error, Fail, Context, Backtrace, ResultExt};
use std::fmt::{self, Display};

// boilerplate error wrapping (might go away)
impl Fail for VaultError {
    fn cause(&self) -> Option<&Fail> { self.inner.cause() }
    fn backtrace(&self) -> Option<&Backtrace> { self.inner.backtrace() }
}
impl Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
//impl VaultError {
//    pub fn kind(&self) -> VErrKind {
//        *self.inner.get_context()
//    }
//}
impl From<VErrKind> for VaultError {
    fn from(kind: VErrKind) -> VaultError {
        VaultError { inner: Context::new(kind) }
    }
}
impl From<Context<VErrKind>> for VaultError {
    fn from(inner: Context<VErrKind>) -> VaultError {
        VaultError { inner: inner }
    }
}
type Result<T> = std::result::Result<T, Error>;

// helpers

fn default_addr() -> Result<String> {
    Ok(env::var("VAULT_ADDR").context(VErrKind::MissingAddr)?)
}

#[cfg(feature = "filesystem")]
fn file_token_fallback() -> Result<String> {
    use std::fs::File;
    let home = dirs::home_dir();
    ensure!(home.is_some(), "system must have a home directory");

    // Read the `.vault-token` file from $HOME
    let mut f = File::open(home.unwrap().join(".vault-token"))?;
    let mut token = String::new();
    f.read_to_string(&mut token)?;
    Ok(token)
}

fn default_token() -> Result<String> {
    let t = env::var("VAULT_TOKEN")
        .or_else(|_: env::VarError| -> Result<String> {
            if cfg!(feature = "filesystem") {
                #[cfg(feature = "filesystem")]
                return file_token_fallback();
            }
            bail!("no vault file outside shipcat cli")
        })
        .context(VErrKind::MissingToken)?;
    Ok(t)
}

/// Secrets in vault values can be integers or strings
///
/// If they are integers, we coerce them to strings
/// This is mostly a convenience because you can't easily quote integers in the UI
/// without them ending up double quoted...
///
/// Use untagged feature to have serde autodetect the type, and implement string coerce.
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum SecretValue {
    S(String),
    I(i64),
}
impl From<SecretValue> for String {
    fn from(sv: SecretValue) -> String {
        match sv {
            SecretValue::I(i) => i.to_string(),
            SecretValue::S(s) => s,
        }
    }
}

/// Secret data retrieved from Vault using only standard fields
#[derive(Debug, Deserialize)]
struct Secret {
    /// The key-value pairs associated with this secret.
    ///
    /// NB: If we put String instead of SecretValue we discard integer-like values
    data: BTreeMap<String, SecretValue>,
    // How long this secret will remain valid for, in seconds.
    lease_duration: u64,
}

/// List data retrieved from Vault when listing available secrets
#[derive(Debug, Deserialize)]
struct ListSecrets {
    data: BTreeMap<String, Vec<String>>
}

/// Vault client with cached data
pub struct Vault {
    /// Our HTTP client.  This can be configured to mock out the network.
    client: reqwest::Client,
    /// The address of our Vault server.
    addr: reqwest::Url,
    /// The token which we'll use to access Vault.
    token: String,
    /// Vault operation mode
    mode: Mode,
}

/// Vault usage mode
#[derive(PartialEq, Debug, Clone)]
pub enum Mode {
    /// Normal HTTP calls to vault returing actual secret
    Standard,
    /// Not using HTTP calls, just returning dummy data
    Mocked,
}

impl Vault {
    /// Initialize using the same evars or token files that the `vault` CLI uses
    pub fn from_evars() -> Result<Vault> {
        Vault::new(reqwest::Client::new(), &default_addr()?, default_token()?, Mode::Standard)
    }

    /// Initialize using VAULT_TOKEN evar + addr in shipcat.conf
    pub fn regional(vc: &VaultConfig) -> Result<Vault> {
        Vault::new(reqwest::Client::new(), &vc.url, default_token()?, Mode::Standard)
    }

    /// Initialize using dummy values and return garbage
    pub fn mocked(vc: &VaultConfig) -> Result<Vault> {
        Vault::new(reqwest::Client::new(), &vc.url, "INVALID_TOKEN".to_string(), Mode::Mocked)
    }

    fn new<U, S>(client: reqwest::Client, addr: U, token: S, mode: Mode) -> Result<Vault>
        where U: reqwest::IntoUrl,
              S: Into<String>
    {
        let addr = addr.into_url()?;
        Ok(Vault { client, addr, mode, token: token.into() })
    }

    pub fn mode(&self) -> Mode {
        self.mode.clone()
    }

    // The actual HTTP GET logic
    fn get_secret(&self, path: &str) -> Result<Secret> {
        let url = self.addr.join(&format!("v1/{}", path))?;
        debug!("GET {}", url);

        let mut res = self.client.get(url.clone())
            .header("X-Vault-Token", self.token.clone())
            .send()
            .context(VErrKind::Url(url.clone()))?;

        // Generate informative errors for HTTP failures, because these can
        // be caused by everything from bad URLs to overly restrictive vault policies
        if !res.status().is_success() {
            let status = res.status().to_owned();
            return Err(VErrKind::UnexpectedHttpStatus(status, url.to_string()))?
        }

        let mut body = String::new();
        res.read_to_string(&mut body)?;
        Ok(serde_json::from_str(&body)?)
    }

    /// List secrets
    ///
    /// Does a HTTP LIST on the folder a service is in and returns the keys
    pub fn list(&self, path: &str) -> Result<Vec<String>> {
        let url = self.addr.join(&format!("v1/secret/{}?list=true", path))?;
        debug!("LIST {}", url);

        let mut res = self.client.get(url.clone())
            .header("X-Vault-Token", self.token.clone())
            .send()
            .context(VErrKind::Url(url.clone()))?;

        // Generate informative errors for HTTP failures, because these can
        // be caused by everything from bad URLs to overly restrictive vault policies
        if !res.status().is_success() {
            let status = res.status().to_owned();
            return Err(VErrKind::UnexpectedHttpStatus(status, url.to_string()))?
        }

        let mut body = String::new();
        res.read_to_string(&mut body)?;
        let lsec : ListSecrets = serde_json::from_str(&body)?;
        if !lsec.data.contains_key("keys") {
            bail!("secret list {} does not contain keys list from vault api!?: {}", url, body);
        }
        let res = lsec.data["keys"].iter()
            .filter(|e| !e.ends_with("/")) // skip sub folders
            .map(|e| e.to_string())
            .collect::<Vec<String>>();
        Ok(res)
    }


    /// Read secret from a Vault via an authenticated HTTP GET (or memory cache)
    pub fn read(&self, key: &str) -> Result<String> {
        let pth = format!("secret/{}", key);
        if self.mode == Mode::Mocked {
            // arbitrary base64 encoded value so it's compatible with everything
            return Ok("aGVsbG8gd29ybGQ=".into());
        }

        let secret = self.get_secret(&pth).context(VErrKind::SecretNotAccessible(pth.clone()))?;

        // NB: Currently assume each path in vault has a single `value`
        // Read the value key (which should exist)
        let s = secret.data
            .get("value")
            .ok_or_else(|| VErrKind::InvalidSecretForm(pth))
            .map(ToOwned::to_owned).map(String::from)?;
        Ok(s)
    }
}


#[cfg(test)]
mod tests {
    use super::Vault;
    use base64;

    #[test]
    fn get_dev_secret() {
        let client = Vault::from_evars().unwrap();
        let secret = client.read("dev-uk/test-shipcat/FAKE_SECRET").unwrap();
        assert_eq!(secret, "hello");

        // integers in vault coerced to strings
        let secretnum = client.read("dev-uk/test-shipcat/FAKE_NUMBER").unwrap();
        assert_eq!(secretnum, "-2");

        // secretfiles are valid base64
        let secretfile = client.read("dev-uk/test-shipcat/fake-file").unwrap();
        assert_eq!(secretfile, "aGVsbG8gd29ybGQgYmFzZTY0Cg==".to_string());
        if let Ok(b) = base64::decode(&secretfile) {
            let s = String::from_utf8(b).unwrap();
            assert_eq!(s, "hello world base64\n");
        } else {
            assert!(false, "fake-file {} in vault is not base64 encoded", secretfile);
        }
    }

    #[test]
    fn list_dev_secrets() {
        let client = Vault::from_evars().unwrap();
        let mut secrets = client.list("dev-uk/test-shipcat").unwrap();
        secrets.sort_unstable(); // ignore key order
        assert_eq!(secrets, vec![
            "FAKE_NUMBER".to_string(),
            "FAKE_SECRET".to_string(),
            "fake-file".to_string()
        ]);
    }
}
