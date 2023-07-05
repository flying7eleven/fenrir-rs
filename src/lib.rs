#![doc = include_str!("../README.md")]

pub mod noop;
#[cfg(feature = "reqwest-async")]
pub mod reqwest;
#[cfg(feature = "ureq")]
pub mod ureq;

#[cfg(feature = "structured_logging")]
use log::kv::{Source, Visitor};
use log::{Log, Metadata, Record};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use url::Url;

/// The [`AuthenticationMethod`] enum is used to specify the authentication method to use when
/// sending the log messages to the remote endpoint.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum AuthenticationMethod {
    /// Do not use any authentication when sending the log messages to the remote endpoint
    None,
    /// Use the HTTP Basic Auth when sending the log messages to the remote endpoint
    Basic,
}

/// The [`NetworkingBackend`] defines all possible networking backends which can be used within
/// the crate.
#[derive(Eq, PartialEq)]
pub enum NetworkingBackend {
    /// The `None` network backend does exactly what it says: it does nothing at all
    None,

    /// The `Ureq` backend uses the `ureq` library for network requests
    #[cfg(feature = "ureq")]
    Ureq,

    /// The `Reqwest` backend uses the `reqwest` library for network requests
    #[cfg(feature = "reqwest-async")]
    Reqwest,
}

impl NetworkingBackend {
    #[cfg(feature = "reqwest-async")]
    pub fn is_async(&self) -> bool {
        matches!(self, NetworkingBackend::Reqwest)
    }

    #[cfg(not(feature = "reqwest-async"))]
    pub fn is_async(&self) -> bool {
        false
    }
}

/// The [`SerializationFormat`] is used to configure the format to which the logging messages should
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
pub(crate) type SerializationFn = fn(&Streams) -> Result<Vec<u8>, String>;

/// This trait is used to specify the interfaces which are required for the communication
/// with the remote endpoint.
pub(crate) trait FenrirBackend {
    /// Sends a `Streams` object to the configured remote backend
    fn send(&self, serialized_stream: Vec<u8>) -> Result<(), String>;

    /// Query the `TypeId` of the implementation of this trait
    fn internal_type(&self) -> std::any::TypeId;

    /// Get the configured `AuthenticationMethod` for the backend
    fn authentication_method(&self) -> AuthenticationMethod;

    /// Get the configured credentials or `None` if no credentials are configured
    fn credentials(&self) -> Option<String>;
}

/// The [`Fenrir`] struct implements the communication interface with a [Loki](https://grafana.com/oss/loki/)
/// instance.
///
/// To create a new instance of the [`Fenrir`] struct use the [`FenrirBuilder`] struct.
pub struct Fenrir {
    backend: Box<dyn FenrirBackend + Send + Sync>,
    additional_tags: HashMap<String, String>,
    serializer: SerializationFn,
    include_level: bool,
    include_framework: bool,
    log_stream: RwLock<Vec<Stream>>,
}

impl Fenrir {
    /// Create a new [`FenrirBuilder`] with all required parameters.
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
            runtime: None,
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
        if module.starts_with("ureq")
            || module.starts_with("reqwest")
            || !self.enabled(record.metadata())
        {
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
        let stream_object = Stream {
            stream: labels,
            values: vec![vec![
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .to_string(),
                record.args().to_string(),
            ]],
        };
        // push the stream object to the log stream
        self.log_stream.write().push(stream_object);
    }

    fn flush(&self) {
        // fetch and serialize the log streams
        let res = {
            // this route can save several allocations since we do not need to clone the streams,
            // and we reuse the allocated memory
            let mut streams = self.log_stream.write();
            let res = (self.serializer)(&Streams { streams: &streams });
            streams.clear();
            res
        };
        match res {
            Ok(serialized_stream) => {
                if let Err(e) = self.backend.send(serialized_stream) {
                    #[cfg(debug_assertions)]
                    panic!("Could not send logs to Loki. The error was: {}", e);
                }
            }
            Err(message) => {
                #[cfg(debug_assertions)]
                panic!("Could not serialize logs. The error was: {}", message);
            }
        }
    }
}

/// The [`FenrirBuilder`] struct is used to create a new instance of [`Fenrir`] using the builder pattern.
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
    /// A runtime handle to the tokio runtime, if it is used
    #[cfg(feature = "async-tokio")]
    runtime: Option<tokio::runtime::Handle>,
    #[cfg(not(feature = "async-tokio"))]
    runtime: Option<()>,
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

    /// Set the network backend which should be used to communicate with a Loki endpoint.
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

    /// Set the runtime handle to the tokio runtime which should be used for sending the log messages.
    ///
    /// # Example
    /// ```
    /// use tokio::runtime::Handle;
    /// use fenrir_rs::Fenrir;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    ///   let handle = Handle::current();
    ///   let builder = Fenrir::builder()
    ///     .tokio_rt_handle(handle);
    /// # }
    #[cfg(feature = "async-tokio")]
    pub fn tokio_rt_handle(mut self, rt_handle: tokio::runtime::Handle) -> FenrirBuilder {
        self.runtime = Some(rt_handle);
        self
    }

    /// Fetch the active runtime's handle and use it as the designated runtime.
    ///
    /// # Note
    /// This method will panic if called outside the context of a Tokio 1.x runtime.
    ///
    /// # Example
    /// ```
    /// use tokio::runtime::Handle;
    /// use fenrir_rs::Fenrir;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    ///   let builder = Fenrir::builder()
    ///     .tokio_rt_handle_current();
    /// # }
    #[cfg(feature = "async-tokio")]
    pub fn tokio_rt_handle_current(mut self) -> FenrirBuilder {
        self.runtime = Some(tokio::runtime::Handle::current());
        self
    }

    /// Create a new `Fenrir` instance with the parameters supplied to this struct before calling this method.
    ///
    /// Before creating a new instance, the supplied parameters are validated (in contrast to [`FenrirBuilder::build`]
    /// which does not validate the supplied parameters).
    ///
    /// # Panics
    /// The method will panic if one or more of the supplied parameters are not valid or seem to have
    /// unintended values.
    ///
    /// # Example
    /// ```should_panic
    /// use url::Url;
    /// use fenrir_rs::{Fenrir, NetworkingBackend, SerializationFormat};
    ///
    /// let fenrir = Fenrir::builder()
    ///     .endpoint(Url::parse("https://loki.example.com").unwrap())
    ///     .network(NetworkingBackend::None)
    ///     .format(SerializationFormat::None)
    ///     .build_with_validation();
    /// ```
    pub fn build_with_validation(self) -> Fenrir {
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

        // panic if no runtime was set and the selected network backend is async
        if self.runtime.is_none() && self.network_backend.is_async() {
            panic!("You have to set a runtime handle before creating an instance of `Fenrir` if you want to use an async network backend");
        }

        // after the validation, we can call the actual build method to create the new Fenrir instance
        self.build()
    }

    /// Create a new `Fenrir` instance with the parameters supplied to this struct before calling this method.
    ///
    /// # Note
    /// If an async network backend is selected, this method will panic if no runtime handle was set,
    /// and this method is called outside the context of a Tokio 1.x runtime.
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

        // create the instance of the required network backend
        let network_backend: Box<dyn FenrirBackend + Send + Sync> = match self.network_backend {
            NetworkingBackend::None => Box::new(NoopBackend {}),

            #[cfg(feature = "ureq")]
            NetworkingBackend::Ureq => Box::new(crate::ureq::UreqBackend {
                authentication: self.authentication,
                credentials: self.credentials,
                endpoint: self.endpoint,
            }),

            #[cfg(feature = "reqwest-async")]
            NetworkingBackend::Reqwest => Box::new(crate::reqwest::ReqwestBackend {
                authentication: self.authentication,
                credentials: self.credentials,
                endpoint: self.endpoint,
                client: ::reqwest::Client::new(),
                runtime_handle: self.runtime.unwrap_or_else(tokio::runtime::Handle::current),
            }),
        };

        // determine the serialization function to use
        let serializer = match self.serialization_format {
            SerializationFormat::None => noop_serializer,

            #[cfg(feature = "json")]
            SerializationFormat::Json => |data: &Streams| -> Result<Vec<u8>, String> {
                serde_json::to_vec(data).map_err(|error| error.to_string())
            },
        };

        // create and return the actual backend
        Fenrir {
            backend: network_backend,
            serializer,
            include_level: self.include_level,
            include_framework: self.include_framework,
            additional_tags: self.additional_tags,
            log_stream: RwLock::new(Vec::new()),
        }
    }
}

/// A serialization implementation which does nothing when requesting to serialize a object
pub(crate) fn noop_serializer(_: &Streams) -> Result<Vec<u8>, String> {
    Ok(vec![])
}

/// A struct for visiting all structured logging labels of a log message and collecting them
#[cfg(feature = "structured_logging")]
struct LokiVisitor<'kvs> {
    values: HashMap<log::kv::Key<'kvs>, log::kv::Value<'kvs>>,
}

/// The implementation for the visitor pattern for collecting all labels attached to a log
/// message
#[cfg(feature = "structured_logging")]
impl<'kvs> LokiVisitor<'kvs> {
    /// Create a new visitor with an initial capacity of `count` labels.
    pub fn new(count: usize) -> Self {
        Self {
            values: HashMap::with_capacity(count),
        }
    }

    /// Read the key-value-pair attached to a log message and collect them
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

/// The [`Visitor'] implementation for the [`LokiVisitor`]
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

/// The data structure used for attaching tags / labels to logging entries before sending them
/// to Loki
#[derive(Serialize)]
pub(crate) struct Stream {
    /// The tags which should be attached to the logging entries
    pub(crate) stream: HashMap<String, String>,
    /// The actual log messages to store with the corresponding meta information
    pub(crate) values: Vec<Vec<String>>,
}

/// The base data structure Loki expects when receiving logging messages.
#[derive(Serialize)]
pub(crate) struct Streams<'a> {
    /// A list of all logging messages with the attached meta information which should be logged
    /// by Loki
    pub(crate) streams: &'a [Stream],
}

#[cfg(test)]
mod tests {
    use crate::{AuthenticationMethod, Fenrir, NetworkingBackend, SerializationFormat};
    use url::Url;

    #[test]
    #[should_panic]
    fn building_a_validated_fenrir_instance_without_network_backend_panics() {
        let fenrir = Fenrir::builder()
            .format(SerializationFormat::Json)
            .build_with_validation();
    }

    #[test]
    fn building_a_non_validated_fenrir_instance_without_network_backend_does_not_panic() {
        let fenrir = Fenrir::builder().format(SerializationFormat::Json).build();
    }

    #[test]
    #[should_panic]
    fn building_a_validated_fenrir_instance_without_serialization_backend_panics() {
        let fenrir = Fenrir::builder()
            .network(NetworkingBackend::Ureq)
            .build_with_validation();
    }

    #[test]
    fn building_a_non_validated_fenrir_instance_without_serialization_backend_does_not_panic() {
        let fenrir = Fenrir::builder().network(NetworkingBackend::Ureq).build();
    }
}
