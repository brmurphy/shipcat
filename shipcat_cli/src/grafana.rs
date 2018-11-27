///
/// Interface for adding grafana annotations about deploys
///

use reqwest;
use chrono::Utc;
use std::env;

/// At what time the annotation should be made
#[derive(Debug)]
pub enum TimeSpec {
    Now,
    Time(u64),
}

/// The type of annotation event
#[derive(Debug)]
pub enum Event {
    Upgrade,
    Rollback,
}

/// A representation of a particular deployment event
#[derive(Debug)]
pub struct Annotation {
    pub event: Event,
    pub service: String,
    pub version: String,
    pub region: String,
    pub time: TimeSpec,
}


// All main errors that can happen from grafana hook

// New failure error type
#[derive(Debug)]
struct GError {
    inner: Context<GErrKind>,
}
// its associated enum
#[derive(Clone, Eq, PartialEq, Debug, Fail)]
enum GErrKind {
    #[fail(display = "GRAFANA_SHIPCAT_HOOK_URL not specified")]
    MissingGrafanaUrl,

    #[fail(display = "GRAFANA_SHIPCAT_TOKEN not specified")]
    MissingGrafanaToken,

    #[fail(display = "could not access URL '{}'", _0)]
    Url(reqwest::Url)
}
use failure::{Error, Fail, Context, Backtrace, ResultExt};
use std::fmt::{self, Display};

// boilerplate error wrapping (might go away)
impl Fail for GError {
    fn cause(&self) -> Option<&Fail> { self.inner.cause() }
    fn backtrace(&self) -> Option<&Backtrace> { self.inner.backtrace() }
}
impl Display for GError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
impl From<GErrKind> for GError {
    fn from(kind: GErrKind) -> GError {
        GError { inner: Context::new(kind) }
    }
}
impl From<Context<GErrKind>> for GError {
    fn from(inner: Context<GErrKind>) -> GError {
        GError { inner: inner }
    }
}
type Result<T> = std::result::Result<T, Error>;


/// Extracts grafana URL + HTTP scheme from environment
pub fn env_hook_url() -> Result<String> {
    Ok(env::var("GRAFANA_SHIPCAT_HOOK_URL").context(GErrKind::MissingGrafanaUrl)?)
}

/// Extracts grafana API key from environment
pub fn env_token() -> Result<String> {
    Ok(env::var("GRAFANA_SHIPCAT_TOKEN").context(GErrKind::MissingGrafanaToken)?)
}

/// Convert timespec to UNIX time, in milliseconds
fn unix_timestamp(spec: &TimeSpec) -> Result<u64> {
  let timestamp = match spec {
    TimeSpec::Now => Utc::now().timestamp_millis() as u64,
    TimeSpec::Time(timestamp) => *timestamp
  };
  Ok(timestamp)
}

/// Create an annotation for a deployment using grafana's REST API
pub fn create(annotation: Annotation) -> Result<()> {
    let hook_url = env_hook_url()?;
    let hook_token = env_token()?;

    let timestamp = unix_timestamp(&annotation.time)?;

    let data = json!({
        "time": timestamp,
        "text": format!("{} {}={} in {}",
            match annotation.event {
                Event::Upgrade => "Upgrade",
                Event::Rollback => "Rollback"
            },
            &annotation.service,
            &annotation.version,
            &annotation.region
        ),
        "tags": [
            "all-deploys",
            format!("{}-deploys", annotation.region),
            format!("{}-deploys", annotation.service)
        ]
    });

    let url = reqwest::Url::parse(&hook_url)?.join("api/annotations")?;
    let client = reqwest::Client::new();

    client.post(url.clone())
        .bearer_auth(hook_token)
        .json(&data)
        .send()
        .context(GErrKind::Url(url.clone()))?;

    Ok(())
}
