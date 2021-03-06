use crate::vault::Vault;
use std::collections::{BTreeMap, BTreeSet};
use regex::Regex;

use crate::config::{Config};
use crate::region::{VaultConfig, Region};
use crate::states::ManifestType;
use super::Result;

// All structs come from the structs directory
use super::structs::{
    {HealthCheck, ConfigMap},
    {InitContainer, Resources, HostAlias},
    volume::{Volume, VolumeMount},
    PersistentVolume,
    {Metadata, VaultOpts, Dependency},
    security::DataHandling,
    Probe,
    {CronJob, Sidecar, EnvVars},
    {Gate, Kafka, Kong, Rbac},
    RollingUpdate,
    autoscaling::AutoScaling,
    tolerations::Tolerations,
    LifeCycle,
    Worker,
    Port,
    rds::Rds,
    elasticache::ElastiCache,
};

/// Main manifest, serializable from shipcat.yml or the shipcat CRD.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    // ------------------------------------------------------------------------
    // Non-mergeable global properties
    //
    // A few global properties that cannot be overridden in region override manifests.
    // These are often not exposed to kube and marked with `skip_serializing`,
    // but more often data that is used internally and assumed global.
    // ------------------------------------------------------------------------

    /// Name of the service
    ///
    /// This must match the folder name in a manifests repository, and additionally;
    /// - length limits imposed by kube dns
    /// - dash separated, alpha numeric names (for dns readability)
    ///
    /// The main validation regex is: `^[0-9a-z\-]{1,50}$`.
    ///
    /// ```yaml
    /// name: webapp
    /// ```
    #[serde(default)]
    pub name: String,

    /// Whether the service should be public
    ///
    /// This is a special flag not exposed to the charts at the moment.
    ///
    /// ```yaml
    /// publiclyAccessible: true
    /// ```
    #[serde(default, skip_serializing)]
    pub publiclyAccessible: bool,

    /// Service is external
    ///
    /// This cancels all validation and marks the manifest as a non-kube reference only.
    ///
    /// ```yaml
    /// external: true
    /// ```
    #[serde(default, skip_serializing)]
    pub external: bool,

    /// Service is disabled
    ///
    /// This disallows usage of this service in all regions.
    ///
    /// ```yaml
    /// disabled: true
    /// ```
    #[serde(default, skip_serializing)]
    pub disabled: bool,

    /// Regions to deploy this service to.
    ///
    /// Every region must be listed in here.
    /// Uncommenting a region in here will partially disable this service.
    #[serde(default, skip_serializing)]
    pub regions: Vec<String>,

    /// Important contacts and other metadata for the service
    ///
    /// Particular uses:
    /// - notifying correct people on upgrades via slack
    /// - providing direct links to code diffs on upgrades in slack
    ///
    /// ```yaml
    /// metadata:
    ///   contacts:
    ///   - name: "Eirik"
    ///     slack: "@clux"
    ///   team: Doves
    ///   repo: https://github.com/clux/blog
    ///   support: "#humans"
    ///   notifications: "#robots"
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    // ------------------------------------------------------------------------
    // Regular mergeable properties
    //
    // New syntax goes in here!
    // All properties in here should be mergeable, so ensure you add merge behaviour.
    // Merge behaviour is defined in the merge module.
    // ------------------------------------------------------------------------


    /// Chart to use for the service
    ///
    /// All the properties in `Manifest` are tailored towards our `base` chart,
    /// so this should be overridden with caution.
    ///
    /// ```yaml
    /// chart: custom
    /// ```
    #[serde(default)]
    pub chart: Option<String>,

    /// Image name of the docker image to run
    ///
    /// This can be left out if imagePrefix is set in the config, and the image name
    /// also matches the service name. Otherwise, this needs to be the full image name.
    ///
    /// ```yaml
    /// image: nginx
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Optional uncompressed image size
    ///
    /// This is used to compute a more accurate wait time for rolling upgrades.
    /// See `Manifest::estimate_wait_time`.
    ///
    /// Ideally, this number is autogenerated from your docker registry.
    ///
    /// ```yaml
    /// imageSize: 1400
    /// ```
    #[serde(skip_serializing)]
    pub imageSize: Option<u32>,

    /// Version aka. tag of docker image to run
    ///
    /// This does not have to be set in "rolling environments", where upgrades
    /// re-use the current running versions. However, for complete control, production
    /// environments should put the versions in manifests.
    ///
    /// Versions must satisfy `VersionScheme::verify`.
    ///
    ///
    /// ```yaml
    /// version: 1.2.0
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Command to use for the docker image
    ///
    /// This can be left out to use the default image command.
    ///
    /// ```yaml
    /// command: ["bundle", "exec", "rake", "jobs:work"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,

    /// Data sources and handling strategies
    ///
    /// An experimental abstraction around GDPR
    ///
    /// ```yaml
    /// dataHandling:
    ///   stores:
    ///   - backend: Postgres
    ///     encrypted: true
    ///     cipher: AES256
    ///     fields:
    ///     - name: BabylonUserId
    ///     - name: HealthCheck
    ///   processes:
    ///   - field: HealthCheck
    ///     source: orchestrator
    /// ```
    #[serde(default, skip_serializing)]
    pub dataHandling: Option<DataHandling>,

    /// Language the service is written in
    ///
    /// This does not provide any special behaviour at the moment.
    ///
    /// ```yaml
    /// language: python
    /// ```
    #[serde(skip_serializing)]
    pub language: Option<String>,


    /// Kubernetes resource limits and requests
    ///
    /// Api straight from [kubernetes resources](https://kubernetes.io/docs/concepts/configuration/manage-compute-resources-container/)
    ///
    /// ```yaml
    /// resources:
    ///   requests:
    ///     cpu: 100m
    ///     memory: 100Mi
    ///   limits:
    ///     cpu: 300m
    ///     memory: 300Mi
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Resources<String>>,

    /// Kubernetes replication count
    ///
    /// This is set on the `Deployment` object in kubernetes.
    /// If you have `autoScaling` parameters set, then these take precedence.
    ///
    /// ```yaml
    /// replicaCount: 4
    /// ```
    #[serde(default)]
    pub replicaCount: Option<u32>,

    /// Environment variables to inject
    ///
    /// These have a few special convenience behaviours:
    /// "IN_VAULT" values is replaced with value from vault/secret/folder/service/KEY
    /// One off `tera` templates are calculated with a limited template context
    ///
    /// IN_VAULT secrets will all be put in a single kubernetes `Secret` object.
    /// One off templates **can** be put in a `Secret` object if marked `| as_secret`.
    ///
    /// ```yaml
    /// env:
    ///   # plain eva:
    ///   PLAIN_EVAR: plaintextvalue
    ///
    ///   # vault lookup:
    ///   DATABASE_URL: IN_VAULT
    ///
    ///   # templated evars:
    ///   INTERNAL_AUTH_URL: "{{ base_urls.services }}/auth/internal"
    ///   AUTH_ID: "{{ kong.consumers['webapp'].oauth_client_id }}"
    ///   AUTH_SECRET: "{{ kong.consumers['webapp'].oauth_client_secret | as_secret }}"
    /// ```
    ///
    /// The vault lookup will GET from the region specific path for vault, in the
    /// webapp subfolder, getting the `DATABASE_URL` secret.
    ///
    /// The `kong` templating will use the secrets read from the `Config` for this
    /// region, and replace them internally.
    ///
    /// The `as_secret` destinction only serves to put `AUTH_SECRET` into `Manifest::secrets`.
    #[serde(default)]
    pub env: EnvVars,


    /// Kubernetes Secret Files to inject
    ///
    /// These have the same special "IN_VAULT" behavior as `Manifest::env`:
    /// "IN_VAULT" values is replaced with value from vault/secret/folder/service/key
    ///
    /// Note the lowercase restriction on keys.
    /// All `secretFiles` are expected to be base64 in vault, and are placed into a
    /// kubernetes `Secret` object.
    ///
    /// ```yaml
    /// secretFiles:
    ///   webapp-ssl-keystore: IN_VAULT
    ///   webapp-ssl-truststore: IN_VAULT
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secretFiles: BTreeMap<String, String>,


    /// Config files to inline in a kubernetes `ConfigMap`
    ///
    /// These are read and templated by `tera` before they are passed to helm.
    /// A full `tera` context from `Manifest::make_template_context` is used.
    ///
    /// ```yaml
    /// configs:
    ///   mount: /config/
    ///   files:
    ///   - name: webhooks.json.j2
    ///     dest: webhooks.json
    ///   - name: newrelic-java.yml.j2
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configs: Option<ConfigMap>,

    /// Vault options
    ///
    /// Allows overriding service names and regions for secrets.
    /// DEPRECATED. Should only be set in rare cases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultOpts>,

    /// Http Port to expose in the kubernetes `Service`
    ///
    /// This is normally the service your application listens on.
    /// Kong deals with mapping the port to a nicer one.
    ///
    /// ```yaml
    /// httpPort: 8000
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub httpPort: Option<u32>,

    /// Ports to open
    ///
    /// For services outside Kong, expose these named ports in the kubernetes `Service`.
    ///
    /// ```yaml
    ///  ports:
    ///  - port: 6121
    ///    name: data
    ///  - port: 6122
    ///    name: rpc
    ///  - port: 6125
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,

    /// Externally exposed port
    ///
    /// Useful for `LoadBalancer` type `Service` objects.
    ///
    /// ```yaml
    /// externalPort: 443
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub externalPort: Option<u32>,

    /// Health check parameters
    ///
    /// A small abstraction around `readinessProbe`.
    /// DEPRECATED. Should use `readinessProbe`.
    ///
    /// ```yaml
    /// health:
    ///   uri: /health
    ///   wait: 15
    /// ```
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<HealthCheck>,

    /// Service dependencies
    ///
    /// Used to construct a dependency graph, and in the case of non-circular trees,
    /// it can be used to arrange deploys in the correct order.
    ///
    /// ```yaml
    /// dependencies:
    /// - name: auth
    /// - name: ask2
    /// - name: chatbot-reporting
    /// - name: clinical-knowledge
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    /// Worker `Deployment` objects to additinally include
    ///
    /// These are more flexible than `sidecars`, because they scale independently of
    /// the main `replicaCount`. However, they are considered separate rolling upgrades.
    /// There is no guarantee that these switch over at the same time as your main
    /// kubernetes `Deployment`.
    ///
    /// ```yaml
    /// workers:
    /// - name: analytics-experiment-taskmanager
    ///   resources:
    ///     limits:
    ///       cpu: 1
    ///       memory: 1Gi
    ///     requests:
    ///       cpu: 250m
    ///       memory: 1Gi
    ///   replicaCount: 3
    ///   preserveEnv: true
    ///   ports:
    ///   - port: 6121
    ///     name: data
    ///   - port: 6122
    ///     name: rpc
    ///   - port: 6125
    ///     name: query
    ///   command: ["/start.sh", "task-manager", "-Djobmanager.rpc.address=analytics-experiment"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<Worker>,

    /// Sidecars to inject into every kubernetes `Deployment`
    ///
    /// Plain sidecars are injected into the main `Deployment` and all the workers' ones.
    /// They scale directly with the sum of `replicaCount`s.
    ///
    /// ```yaml
    /// sidecars:
    /// - name: redis
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecars: Vec<Sidecar>,

    /// `readinessProbe` for kubernetes
    ///
    /// This configures the service's health check, which is used to gate rolling upgrades.
    /// Api is a direct translation of [kubernetes liveness/readiness probes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/).
    ///
    /// This replaces shipcat's `Manifest::health` abstraction.
    ///
    /// ```yaml
    /// readinessProbe:
    ///   httpGet:
    ///     path: /
    ///     port: http
    ///     httpHeaders:
    ///     - name: X-Forwarded-Proto
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readinessProbe: Option<Probe>,

    /// `livenessProbe` for kubernetes
    ///
    /// This configures a `livenessProbe` check. Similar to `readinessProbe`, but with the instruction to kill the pod on failure.
    /// Api is a direct translation of [kubernetes liveness/readiness probes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/).
    ///
    /// ```yaml
    /// livenessProbe:
    ///   tcpSocket:
    ///     port: redis
    ///   initialDelaySeconds: 15
    ///   periodSeconds: 15
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livenessProbe: Option<Probe>,

    /// Container lifecycle events for kubernetes
    ///
    /// This allows commands to be executed either `postStart` or `preStop`
    /// https://kubernetes.io/docs/tasks/configure-pod-container/attach-handler-lifecycle-event/
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<LifeCycle>,

    /// Rolling update Deployment parameters
    ///
    /// These tweak the speed and care kubernetes uses when doing a rolling update.
    /// Sraight from [kubernetes rolling update parameters](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#rolling-update-deployment).
    /// This is attached onto the main `Deployment`.
    ///
    /// ```yaml
    /// rollingUpdate:
    ///   maxUnavailable: 0%
    ///   maxSurge: 50%
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollingUpdate: Option<RollingUpdate>,

    /// `HorizontalPodAutoScaler` parameters for kubernetes
    ///
    /// Passed all parameters directly onto the `spec` of a kube HPA.
    /// Straight from [kubernetes horizontal pod autoscaler](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/).
    ///
    /// ```yaml
    /// autoScaling:
    ///   minReplicas: 6
    ///   maxReplicas: 9
    ///   metrics:
    ///   - type: Resource
    ///     resource:
    ///       name: cpu
    ///       targetAverageUtilization: 60
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoScaling: Option<AutoScaling>,

    /// Toleration parameters for kubernetes
    ///
    /// Bind a service to a particular type of kube `Node`.
    /// Straight from [kubernetes taints and tolerations](https://kubernetes.io/docs/concepts/configuration/taint-and-toleration/).
    ///
    /// ```yaml
    /// tolerations:
    /// - key: "dedicated"
    ///   operator: "Equal"
    ///   value: "hugenode"
    ///   effect: "NoSchedule"
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerations: Vec<Tolerations>,

    /// Host aliases to inject in /etc/hosts in every kubernetes `Pod`
    ///
    /// Straight from [kubernetes host aliases](https://kubernetes.io/docs/concepts/services-networking/add-entries-to-pod-etc-hosts-with-host-aliases/).
    ///
    /// ```yaml
    /// hostAliases:
    /// - ip: "160.160.160.160"
    ///   hostnames:
    ///   - weird-service.babylontech.co.uk
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostAliases: Vec<HostAlias>,

    /// `initContainer` list for every kubernetes `Pod`
    ///
    /// Allows database connectivity checks to be done as pre-boot init-step.
    /// Straight frok [kubernetes init containers](https://kubernetes.io/docs/concepts/workloads/pods/init-containers/).
    ///
    /// ```yaml
    /// initContainers:
    /// - name: init-cassandra
    ///   image: gophernet/netcat
    ///   command: ['sh', '-c', 'until nc -z dev-cassandra 9042; do sleep 2; done;']
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initContainers: Vec<InitContainer>,

    /// Volumes that can be mounted in every kubernetes `Pod`
    ///
    /// Supports our subset of [kubernetes volumes](https://kubernetes.io/docs/concepts/storage/volumes/)
    ///
    /// ```yaml
    /// volumes:
    /// - name: google-creds
    ///   secret:
    ///     secretName: google-creds
    ///     items:
    ///     - key: file
    ///       path: google-cloud-creds.json
    ///       mode: 292
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,

    /// Volumes to mount to every kubernetes `Pod`
    ///
    /// Requires the `Manifest::volumes` entries.
    /// Straight from [kubernetes volumes](https://kubernetes.io/docs/concepts/storage/volumes/)
    ///
    /// ```yaml
    /// volumeMounts:
    /// - name: ssl-store-files
    ///   mountPath: /conf/ssl/
    ///   readOnly: true
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumeMounts: Vec<VolumeMount>,

    /// PersistentVolume injected in helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    /// Straight from [kubernetes volumes](https://kubernetes.io/docs/concepts/storage/volumes/)
    ///
    /// ```yaml
    /// persistentVolumes:
    /// - name: data
    ///   claim: mysql
    ///   storageClass: "gp2"
    ///   accessMode: ReadWriteOnce
    ///   size: 10Gi
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub persistentVolumes: Vec<PersistentVolume>,

    /// Cronjobs images to run as kubernetes `CronJob` objects
    ///
    /// Limited usefulness abstraction, that should be avoided.
    /// An abstraction on top of [kubernetes cron jobs](https://kubernetes.io/docs/concepts/workloads/controllers/cron-jobs/)
    ///
    /// ```yaml
    /// cronJobs:
    /// - name: webapp-promotions-expire
    ///   schedule: "1 0 * * *"
    ///   command: ["bundle", "exec", "rake", "cron:promotions:expire", "--silent"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cronJobs: Vec<CronJob>,

    /// Annotations to set on `Service` objects
    ///
    /// Useful for `LoadBalancer` type `Service` objects.
    /// Not useful for kong balanced services.
    ///
    /// ```yaml
    /// serviceAnnotations:
    ///   svc.k8s.io/aws-load-balancer-ssl-cert: arn:aws:acm:eu-west-2:12345:certificate/zzzz
    ///   svc.k8s.io/aws-load-balancer-backend-protocol: http
    ///   svc.k8s.io/aws-load-balancer-ssl-ports: "443"
    ///   svc.k8s.io/aws-load-balancer-ssl-negotiation-policy: ELBSecurityPolicy-TLS-1-2-2018-01
    ///   helm.sh/resource-policy: keep
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub serviceAnnotations: BTreeMap<String, String>,

    /// Labels for every kubernetes object
    ///
    /// Injected in all top-level kubernetes object as a prometheus convenience.
    /// https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/
    ///
    /// ```yaml
    /// labels:
    ///   custom-metrics: true
    /// ```
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,

    /// Kong config
    ///
    /// A mostly straight from API configuration struct for Kong
    /// Work in progress. `structs::kongfig` contain the newer abstractions.
    ///
    /// ```yaml
    /// kong:
    ///   uris: /webapp
    ///   strip_uri: true
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kong: Option<Kong>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,

    /// Hosts to override kong hosts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,

    /// Kafka config
    ///
    /// A small convencience struct to indicate that the service uses `Kafka`.
    /// The chart will inject a few environment variables and a kafka initContainer
    /// if this is set to a `Some`.
    ///
    /// ```yaml
    /// kafka: {}
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kafka: Option<Kafka>,

    /// Load balancer source ranges
    ///
    /// This is useful for charts that expose a `Service` of `LoadBalancer` type.
    /// IP CIDR ranges, which Kubernetes will use to configure firewall exceptions.
    ///
    /// ```yaml
    /// sourceRanges:
    /// - 0.0.0.0/0
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sourceRanges: Vec<String>,

    /// Role-Based Access Control
    ///
    /// A list of resources to allow the service access to use.
    /// This is a subset of kubernetes `Role::rules` parameters.
    ///
    /// ```yaml
    /// rbac:
    /// - apiGroups: ["extensions"]
    ///   resources: ["deployments"]
    ///   verbs: ["get", "watch", "list"]
    /// ```
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rbac: Vec<Rbac>,

    /// Database provisioning sent to terraform
    ///
    /// Set the base parameters for an RDS instance.
    ///
    /// ```yaml
    /// database:
    ///   engine: postgres
    ///   version: 9.6
    ///   size: 20
    ///   instanceClass: "db.m4.large"
    /// ```
    pub database: Option<Rds>,

    /// Redis provisioning sent to terraform
    ///
    /// Set the base parameters for an ElastiCache instance
    ///
    /// ```yaml
    /// redis:
    ///   nodes: 2
    ///   nodeType: cache.m4.large
    /// ```
    pub redis: Option<ElastiCache>,

    // ------------------------------------------------------------------------
    // Output variables
    //
    // Properties here cannot be deserialized and are illegal in manifests!
    // if you add anything below here, you need to handle behaviour for it.
    // These must be marked with `skip_deserializing` serde attributes.
    // ------------------------------------------------------------------------

    /// Region injected into helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default)]
    #[cfg_attr(filesystem, serde(skip_deserializing))]
    pub region: String,

    /// Environment injected into the helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default)]
    #[cfg_attr(filesystem, serde(skip_deserializing))]
    pub environment: String,

    /// Namespace injected in helm chart
    ///
    /// Exposed from shipcat, but not overrideable.
    #[serde(default)]
    #[cfg_attr(filesystem, serde(skip_deserializing))]
    pub namespace: String,

    /// Raw secrets from environment variables.
    ///
    /// The `env` map fills in secrets in this via the `vault` client.
    /// `Manifest::secrets` partitions `env` into `env` and `secrets`.
    /// See `Manifest::env`.
    ///
    /// This is an internal property that is exposed as an output only.
    #[serde(default, skip_deserializing, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,

    /// Internal kind of the manifest
    ///
    /// A manifest goes through different stages of serialization, templating,
    /// config loading, secret injection. This property keeps track of it.
    #[serde(default, skip_deserializing, skip_serializing)]
    pub kind: ManifestType,
}

impl Manifest {
    /// Override version with an optional one from the CLI
    pub fn set_version(mut self, ver: &Option<String>) -> Self {
        if ver.is_some() {
            self.version = ver.clone(); // override version here if set
        }
        self
    }

    /// Print manifest to stdout
    pub fn print(&self) -> Result<()> {
        println!("{}", serde_yaml::to_string(self)?);
        Ok(())
    }

    /// Verify assumptions about manifest
    ///
    /// Assumes the manifest has been populated with `implicits`
    pub fn verify(&self, conf: &Config, region: &Region) -> Result<()> {
        assert!(self.region != ""); // needs to have been set by implicits!
        if !self.regions.contains(&self.region.to_string()) {
            bail!("Unsupported region {} for service {}", self.region, self.name);
        }
        // limit to 50 characters, alphanumeric, dashes for sanity.
        // 63 is kube dns limit (13 char suffix buffer)
        let re = Regex::new(r"^[0-9a-z\-]{1,50}$").unwrap();
        if !re.is_match(&self.name) {
            bail!("Please use a short, lower case service names with dashes");
        }
        if self.name.ends_with('-') || self.name.starts_with('-') {
            bail!("Please use dashes to separate words only");
        }

        if let Some(ref dh) = self.dataHandling {
            dh.verify()?
        } // TODO: mandatory for later environments!

        if let Some(ref md) = self.metadata {
            md.verify(&conf.teams)?;
        } else {
            bail!("Missing metadata for {}", self.name);
        }

        if self.external {
            warn!("Ignoring most validation for kube-external service {}", self.name);
            return Ok(());
        }

        if let Some(v) = &self.version {
            region.versioningScheme.verify(v)?;
        }

        // TODO [DIP-499]: Separate gate/kong params + adjust the checks
        if let Some(g) = &self.gate {
            if self.kong.is_none() {
                bail!("Can't have a `gate` configuration without a `kong` one");
            }
            if g.public != self.publiclyAccessible {
                bail!("[Migration plan] `publiclyAccessible` and `gate.public` must be equal");
            }
        }

        // run the `Verify` trait on all imported structs
        // mandatory structs first
        if let Some(ref r) = self.resources {
            r.verify()?;
        } else {
            bail!("Resources is mandatory");
        }

        // optional/vectorised entries
        for d in &self.dependencies {
            d.verify()?;
        }
        for ha in &self.hostAliases {
            ha.verify()?;
        }
        for tl in &self.tolerations {
            tl.verify()?;
        }
        for ic in &self.initContainers {
            ic.verify()?;
        }
        for wrk in &self.workers {
            wrk.verify()?;
        }
        for s in &self.sidecars {
            s.verify()?;
        }
        for c in &self.cronJobs {
            c.verify()?;
        }
        for p in &self.ports {
            p.verify()?;
        }
        for r in &self.rbac {
            r.verify()?;
        }
        for pv in &self.persistentVolumes {
            pv.verify()?;
        }
        if let Some(ref cmap) = self.configs {
            cmap.verify()?;
        }
        // misc minor properties
        if self.replicaCount.unwrap() == 0 {
            bail!("Need replicaCount to be at least 1");
        }
        if let Some(ref ru) = &self.rollingUpdate {
            ru.verify(self.replicaCount.unwrap())?;
        }

        self.env.verify()?;

        // internal errors - implicits set these!
        if self.image.is_none() {
            bail!("Image should be set at this point")
        }
        if self.imageSize.is_none() {
            bail!("imageSize must be set at this point");
        }
        if self.chart.is_none() {
            bail!("chart must be set at this point");
        }
        if self.namespace == "" {
            bail!("namespace must be set at this point");
        }
        if self.regions.is_empty() {
            bail!("No regions specified for {}", self.name);
        }
        if self.environment == "" {
            bail!("Service {} ended up with an empty environment", self.name);
        }
        if self.namespace == "" {
            bail!("Service {} ended up with an empty namespace", self.name);
        }

        // health check
        // every service that exposes http MUST have a health check
        if self.httpPort.is_some() && (self.health.is_none() && self.readinessProbe.is_none()) {
            bail!("{} has an httpPort but no health check", self.name)
        }

        // add some warnigs about missing health checks and ports regardless
        // TODO: make both mandatory once we have sidecars supported
        if self.httpPort.is_none() {
            warn!("{} exposes no http port", self.name);
        }
        if self.health.is_none() && self.readinessProbe.is_none() {
            warn!("{} does not set a health check", self.name)
        }

        if !self.serviceAnnotations.is_empty() {
            warn!("serviceAnnotation is an experimental/temporary feature")
        }
        if let Some(db) = &self.database {
            db.verify()?;
        }
        if let Some(redis) = &self.redis {
            redis.verify()?;
        }

        Ok(())
    }

    fn get_vault_path(&self, vc: &VaultConfig) -> String {
        // some services use keys from other services
        let (svc, reg) = if let Some(ref vopts) = self.vault {
            (vopts.name.clone(), vopts.region.clone().unwrap_or_else(|| vc.folder.clone()))
        } else {
            (self.name.clone(), vc.folder.clone())
        };
        format!("{}/{}", reg, svc)
    }

    // Get EnvVars for all containers, workers etc. for this Manifest.
    pub fn get_env_vars(&mut self) -> Vec<&mut EnvVars> {
        let mut envs = Vec::new();
        envs.push(&mut self.env);
        for s in &mut self.sidecars {
            envs.push(&mut s.env);
        }
        for w in &mut self.workers {
            envs.push(&mut w.env);
        }
        for c in &mut self.cronJobs {
            envs.push(&mut c.env);
        }
        envs
    }

    /// Populate placeholder fields with secrets from vault
    ///
    /// This will use the HTTP api of Vault using the configuration parameters
    /// in the `Config`.
    pub fn secrets(&mut self, client: &Vault, vc: &VaultConfig) -> Result<()> {
        let pth = self.get_vault_path(vc);
        debug!("Injecting secrets from vault {} ({:?})", pth, client.mode());

        let mut vault_secrets = BTreeSet::new();
        let mut template_secrets = BTreeMap::new();
        for e in &mut self.get_env_vars() {
            for k in e.vault_secrets() {
                vault_secrets.insert(k.to_string());
            }
            for (k, v) in e.template_secrets() {
                let original = template_secrets.insert(k.to_string(), v.to_string());
                if original.iter().any(|x| x == &v) {
                    bail!("Secret {} can not be used in multiple templates with different values", k);
                }
            }
        }

        let template_keys = template_secrets.keys().map(|x| x.to_string()).collect();
        if let Some(k) = vault_secrets.intersection(&template_keys).next() {
            bail!("Secret {} can not be both templated and fetched from vault", k);
        }

        // Lookup values for each secret in vault.
        for k in vault_secrets {
            let vkey = format!("{}/{}", pth, k);
            self.secrets.insert(k.to_string(), client.read(&vkey)?);
        }

        self.secrets.append(&mut template_secrets);

        // do the same for secret secrets
        for (k, v) in &mut self.secretFiles {
            if v == "IN_VAULT" {
                let vkey = format!("{}/{}", pth, k);
                *v = client.read(&vkey)?;
            }
            // sanity check; secretFiles are assumed base64 verify we can decode
            if base64::decode(v).is_err() {
                bail!("Secret {} is not base64 encoded", k);
            }
        }
        Ok(())
    }

    /// Get a list of raw secrets (without associated keys)
    ///
    /// Useful for obfuscation mechanisms so it knows what to obfuscate.
    pub fn get_secrets(&self) -> Vec<String> {
        self.secrets.values().cloned().collect()
    }

    pub fn verify_secrets_exist(&self, vc: &VaultConfig) -> Result<()> {
        // what are we requesting
        // TODO: Use envvars directly
        let keys = self
            .env
            .plain
            .clone()
            .into_iter()
            .filter(|(_, v)| v == "IN_VAULT")
            .map(|(k, _)| k)
            .collect::<Vec<_>>();
        let files = self.secretFiles.clone().into_iter()
            .filter(|(_,v)| v == "IN_VAULT")
            .map(|(k, _)| k)
            .collect::<Vec<_>>();
        if keys.is_empty() && files.is_empty() {
            return Ok(()); // no point trying to cross reference
        }

        // what we have
        let v = Vault::regional(vc)?; // only listing anyway
        let secpth = self.get_vault_path(vc);
        let found = v.list(&secpth)?; // can fail if folder is empty
        debug!("Found secrets {:?} for {}", found, self.name);

        // compare
        for k in keys {
            if !found.contains(&k) {
                bail!("Secret {} not found in vault {} for {}", k, secpth, self.name);
            }
        }
        for k in files {
            if !found.contains(&k) {
                bail!("Secret file {} not found in vault {} for {}", k, secpth, self.name);
            }
        }
        Ok(())
    }
}
