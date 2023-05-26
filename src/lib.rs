#![doc = include_str!("../README.md")]

pub mod noop;
#[cfg(feature = "ureq")]
pub mod ureq;

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
pub enum NetworkingBackend {
    /// The `None` network backend does exactly what it says: it does nothing at all
    None,

    /// The `Ureq` backend uses the `ureq` library for network requests
    #[cfg(feature = "ureq")]
    Ureq,
}

/// The `SerializationFormat` is used to configure the format to which the logging messages should
/// be serialized to before sending them to the Loki endpoint.
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
    serializer: SerializationFn,
}

impl Fenrir {
    /// Create a new `FenrirBuilder` with all required parameters.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    /// use fenrir_rs::noop::NoopBackend;
    ///
    /// let builder = Fenrir::builder();
    /// ```
    pub fn builder() -> FenrirBuilder {
        FenrirBuilder {
            endpoint: Url::parse("http://localhost:3100").unwrap(),
            authentication: AuthenticationMethod::None,
            network_backend: NetworkingBackend::None,
            serialization_format: SerializationFormat::None,
            credentials: "".to_string(),
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

        // create the logging stream we want to send to loki
        let log_stream = Streams {
            streams: vec![Stream {
                stream: HashMap::from([
                    ("logging_framework".to_string(), "fenrir".to_string()),
                    ("level".to_string(), record.level().to_string()),
                ]),
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
        self.backend.send(&log_stream, self.serializer).unwrap();
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
    /// The `credentials` to use to authenticate against the remote `endpoint`
    credentials: String,
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

    /// Create a new `Fenrir` instance with the parameters supplied to this struct before calling `build()`.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::{Fenrir, NetworkingBackend};
    ///
    /// let fenrir = Fenrir::builder()
    ///     .endpoint(Url::parse("https://loki.example.com").unwrap())
    ///     .network(NetworkingBackend::None)
    ///     .build();
    /// ```
    pub fn build(self) -> Fenrir {
        use crate::noop::NoopBackend;
        #[cfg(feature = "ureq")]
        use crate::ureq::UreqBackend;

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

        // create and retun the actual backend
        Fenrir {
            backend: network_backend,
            serializer,
        }
    }
}

pub(crate) fn noop_serializer(_: &Streams) -> Result<String, String> {
    Ok("".to_string())
}

#[derive(Serialize)]
pub(crate) struct Stream {
    pub(crate) stream: HashMap<String, String>,
    pub(crate) values: Vec<Vec<String>>,
}

#[derive(Serialize)]
pub(crate) struct Streams {
    pub(crate) streams: Vec<Stream>,
}
