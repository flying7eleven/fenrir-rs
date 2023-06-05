#![doc = include_str!("../README.md")]

pub mod noop;
#[cfg(feature = "ureq")]
pub mod ureq;

#[cfg(feature = "structured_logging")]
use log::kv::{Source, Visitor};
use log::{Log, Metadata, Record};
use serde::Serialize;
use std::collections::HashMap;
use url::Url;

/// The `AuthenticationMethod` enum is used to specify the authentication method to use when
/// sending the log messages to the remote endpoint.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum AuthenticationMethod {
    /// Do not use any authentication when sending the log messages to the remote endpoint
    None,
    /// Use the HTTP Basic Auth when sending the log messages to the remote endpoint
    Basic,
}

/// The `NetworkingBackend` defines all possible networking backends which can be used within
/// the crate.
#[derive(Eq, PartialEq)]
pub enum NetworkingBackend {
    /// The `None` network backend does exactly what it says: it does nothing at all
    None,

    /// The `Ureq` backend uses the `ureq` library for network requests
    #[cfg(feature = "ureq")]
    Ureq,
}

/// The `SerializationFormat` is used to configure the format to which the logging messages should
/// be serialized to before sending them to the Loki endpoint.
#[derive(Eq, PartialEq)]
pub enum SerializationFormat {
    /// Do not serialize the data at all
    None,

    /// Use JSON as the serialization format
    #[cfg(feature = "json")]
    Json,
}

/// The function definition which is used to serialize the logging messages for Loki
pub(crate) type SerializationFn = fn(&Streams) -> Result<String, String>;

/// The `FenrirBackend` trait is used to specify the interfaces which are required for the communication
/// with the remote endpoint.
pub(crate) trait FenrirBackend {
    /// Sends a `Streams` object to the configured remote backend
    fn send(&self, streams: &Streams, serializer: SerializationFn) -> Result<(), String>;

    /// Query the `TypeId` of the implementation of this trait
    fn internal_type(&self) -> std::any::TypeId;

    /// Get the configured `AuthenticationMethod` for the backend
    fn authentication_method(&self) -> AuthenticationMethod;

    /// Get the configured credentials or `None` if no credentials are configured
    fn credentials(&self) -> Option<String>;
}

/// The `Fenrir` struct implements the communication interface with a [Loki](https://grafana.com/oss/loki/)
/// instance.
///
/// To create a new instance of the `Fenrir` struct use the `FenrirBuilder` struct.
pub struct Fenrir {
    backend: Box<dyn FenrirBackend + Send + Sync>,
    additional_tags: HashMap<String, String>,
    serializer: SerializationFn,
    include_level: bool,
    include_framework: bool,
}

impl Fenrir {
    /// Create a new `FenrirBuilder` with all required parameters.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    ///
    /// let builder = Fenrir::builder();
    /// ```
    pub fn builder() -> FenrirBuilder {
        FenrirBuilder {
            endpoint: Url::parse("http://localhost:3100").unwrap(),
            authentication: AuthenticationMethod::None,
            network_backend: NetworkingBackend::None,
            serialization_format: SerializationFormat::None,
            additional_tags: HashMap::new(),
            credentials: "".to_string(),
            include_level: false,
            include_framework: false,
        }
    }
}

impl Log for Fenrir {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true // TODO: use the metadata object to decide if we should be enabled or not
    }

    fn log(&self, record: &Record) {
        use std::time::{SystemTime, UNIX_EPOCH};

        // we do want to ignore logs which are created by the used networking library since this
        // would create an infinite loop
        // TODO: this check should move into the backend implementation
        let module = record.module_path().unwrap_or("");
        if module.starts_with("ureq") || !self.enabled(record.metadata()) {
            return;
        }

        // a map with all labels which should be attached to the log entries
        let mut labels = HashMap::new();

        // the default labels supplied with all entries
        if self.include_framework {
            labels.insert("logging_framework".to_string(), "fenrir".to_string());
        }
        if self.include_level {
            labels.insert("level".to_string(), record.level().to_string());
        }

        // add the additional tags to the labels (this might overwrite existing labels)
        labels.extend(self.additional_tags.clone());

        // if structured logging is enabled, add the labels which were attached at the single entries
        #[cfg(feature = "structured_logging")]
        {
            let kv = record.key_values();
            let mut visitor = LokiVisitor::new(kv.count());
            let values = visitor.read_kv(kv).unwrap();

            labels.extend(
                values
                    .iter()
                    .map(|(key, value)| (key.to_string(), value.to_string())),
            );
        }

        // create the logging stream we want to send to loki
        let log_stream = Streams {
            streams: vec![Stream {
                stream: labels,
                values: vec![vec![
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                        .to_string(),
                    record.args().to_string(),
                ]],
            }],
        };

        // send the log stream using the configured backend
        match self.backend.send(&log_stream, self.serializer) {
            Ok(_) => { /* nothing to do here*/ }
            Err(_message) => {
                #[cfg(debug_assertions)]
                panic!("Could not send logs to Loki. The error was: {}", _message);
            }
        }
    }

    fn flush(&self) {
        // TODO: implement the actual flushing
    }
}

/// The `FenrirBuilder` struct is used to create a new instance of `Fenrir`using the builder pattern.
///
/// This should make it easier and more intuitive (at least I hope) to use the crate without referring
/// to the documentation all the time.
pub struct FenrirBuilder {
    /// The loki `endpoint` which is used to send log information to
    endpoint: Url,
    /// The `authentication` method to use when sending the log messages to the remote endpoint
    authentication: AuthenticationMethod,
    /// The `network_backend` which should be used for the network requests
    network_backend: NetworkingBackend,
    /// The `serialization_format´ used for the logging messages
    serialization_format: SerializationFormat,
    /// A map of additional tags which should be attached to all log messages
    additional_tags: HashMap<String, String>,
    /// The `credentials` to use to authenticate against the remote `endpoint`
    credentials: String,
    /// If set to `true`, the logging level is included as a tag
    include_level: bool,
    /// If set to `true`, the logging framework (`fenrir-rs`) is included as a tag
    include_framework: bool,
}

impl FenrirBuilder {
    /// Set the endpoint which should be used for sending the log messages to.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    ///
    /// let builder = Fenrir::builder()
    ///     .endpoint(Url::parse("https://loki.example.com").unwrap());
    /// ```
    pub fn endpoint(mut self, endpoint: Url) -> FenrirBuilder {
        self.endpoint = endpoint;
        self
    }

    /// TODO
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::{Fenrir, NetworkingBackend};
    ///
    /// let builder = Fenrir::builder()
    ///     .network(NetworkingBackend::None);
    /// ```
    pub fn network(mut self, backend: NetworkingBackend) -> FenrirBuilder {
        self.network_backend = backend;
        self
    }

    /// Ensure our client uses the supplied credentials for authentication against the remote endpoint.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::{AuthenticationMethod, Fenrir};
    ///
    /// let builder = Fenrir::builder()
    ///     .with_authentication(AuthenticationMethod::Basic, "foo".to_string(), "bar".to_string());
    /// ```
    pub fn with_authentication(
        mut self,
        method: AuthenticationMethod,
        username: String,
        password: String,
    ) -> FenrirBuilder {
        match method {
            AuthenticationMethod::None => {}
            AuthenticationMethod::Basic => {
                use base64::{engine::general_purpose, Engine};
                let b64_credentials =
                    general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                self.credentials = b64_credentials;
            }
        }

        self.authentication = method;
        self
    }

    /// Select the format which should be used for serializing the logging messages before sending
    /// them to the configured Loki endpoint.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::{SerializationFormat, Fenrir};
    ///
    /// let builder = Fenrir::builder()
    ///     .format(SerializationFormat::None);
    /// ```
    pub fn format(mut self, format: SerializationFormat) -> FenrirBuilder {
        self.serialization_format = format;
        self
    }

    /// Add an additional tag to all logging messages which are sent to Loki.
    /// This can be used to add additional information to the log messages which can be used for
    /// filtering in Loki.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    ///
    /// let builder = Fenrir::builder()
    ///     .tag("service", "service_name")
    ///     .tag("environment", "production");
    /// ```
    pub fn tag(mut self, name: &str, value: &str) -> FenrirBuilder {
        self.additional_tags
            .insert(name.to_string(), value.to_string());
        self
    }

    /// Ensure that a tag for the logging level of the logging message is included in each logging
    /// message send to Loki.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    ///
    /// let builder = Fenrir::builder()
    ///     .include_level();
    /// ```
    pub fn include_level(mut self) -> FenrirBuilder {
        self.include_level = true;
        self
    }

    /// Ensure that a tag for the used logging framework (`fenrir-rs`) is included in all send
    /// logging messages.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    ///
    /// let builder = Fenrir::builder()
    ///     .include_framework();
    /// ```
    #[deprecated(
        since = "0.4.1",
        note = "This is not useful in general and can be achieved by using `tag` method."
    )]
    pub fn include_framework(mut self) -> FenrirBuilder {
        self.include_framework = true;
        self
    }

    /// Create a new `Fenrir` instance with the parameters supplied to this struct before calling `build()`.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::{Fenrir, NetworkingBackend, SerializationFormat};
    ///
    /// let fenrir = Fenrir::builder()
    ///     .endpoint(Url::parse("https://loki.example.com").unwrap())
    ///     .network(NetworkingBackend::Ureq)
    ///     .format(SerializationFormat::Json)
    ///     .build();
    /// ```
    pub fn build(self) -> Fenrir {
        use crate::noop::NoopBackend;
        #[cfg(feature = "ureq")]
        use crate::ureq::UreqBackend;

        // panic if no network backend was selected
        if self.network_backend == NetworkingBackend::None {
            panic!(
                "You have to select a `NetworkingBackend` before creating an instance of `Fenrir`"
            );
        }

        // panic if no serialization format was selected
        if self.serialization_format == SerializationFormat::None {
            panic!("You have to select a `SerializationFormat` before creating an instance of `Fenrir`");
        }

        // create the instance of the required network backend
        let network_backend: Box<dyn FenrirBackend + Send + Sync> = match self.network_backend {
            NetworkingBackend::None => Box::new(NoopBackend {}),

            #[cfg(feature = "ureq")]
            NetworkingBackend::Ureq => Box::new(UreqBackend {
                authentication: self.authentication,
                credentials: self.credentials,
                endpoint: self.endpoint,
            }),
        };

        // determine the serialization function to use
        let serializer = match self.serialization_format {
            SerializationFormat::None => noop_serializer,

            #[cfg(feature = "json")]
            SerializationFormat::Json => |data: &Streams| -> Result<String, String> {
                serde_json::to_string(data).map_err(|error| error.to_string())
            },
        };

        // create and return the actual backend
        Fenrir {
            backend: network_backend,
            serializer,
            include_level: self.include_level,
            include_framework: self.include_framework,
            additional_tags: self.additional_tags,
        }
    }
}

/// TODO: document this
pub(crate) fn noop_serializer(_: &Streams) -> Result<String, String> {
    Ok("".to_string())
}

/// TODO: document this
#[cfg(feature = "structured_logging")]
struct LokiVisitor<'kvs> {
    values: HashMap<log::kv::Key<'kvs>, log::kv::Value<'kvs>>,
}

/// TODO: document this
#[cfg(feature = "structured_logging")]
impl<'kvs> LokiVisitor<'kvs> {
    /// TODO: document this
    pub fn new(count: usize) -> Self {
        Self {
            values: HashMap::with_capacity(count),
        }
    }

    /// TODO: document this
    pub fn read_kv(
        &'kvs mut self,
        source: &'kvs dyn Source,
    ) -> Result<&HashMap<log::kv::Key<'kvs>, log::kv::Value<'kvs>>, log::kv::Error> {
        for _ in 0..source.count() {
            source.visit(self)?;
        }
        Ok(&self.values)
    }
}

/// TODO: document this
#[cfg(feature = "structured_logging")]
impl<'kvs> Visitor<'kvs> for LokiVisitor<'kvs> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        self.values.insert(key, value);
        Ok(())
    }
}

/// TODO: document this
#[derive(Serialize)]
pub(crate) struct Stream {
    /// TODO: document this
    pub(crate) stream: HashMap<String, String>,
    /// TODO: document this
    pub(crate) values: Vec<Vec<String>>,
}

/// TODO: document this
#[derive(Serialize)]
pub(crate) struct Streams {
    /// TODO: document this
    pub(crate) streams: Vec<Stream>,
}

#[cfg(test)]
mod tests {
    use crate::{AuthenticationMethod, Fenrir, NetworkingBackend, SerializationFormat};
    use url::Url;

    #[test]
    #[should_panic]
    fn building_a_fenrir_instance_without_network_backend_panics() {
        let fenrir = Fenrir::builder().format(SerializationFormat::Json).build();
    }

    #[test]
    #[should_panic]
    fn building_a_fenrir_instance_without_serialization_backend_panics() {
        let fenrir = Fenrir::builder().network(NetworkingBackend::Ureq).build();
    }
}
