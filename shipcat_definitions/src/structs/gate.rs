/// Gate setup for a service
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    #[serde(default)]
    pub websockets: bool,
    #[serde(default)]
    pub public: bool,
}
