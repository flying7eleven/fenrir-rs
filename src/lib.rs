#![doc = include_str!("../README.md")]

pub mod noop;
#[cfg(feature = "ureq")]
pub mod ureq;

use log::{Log, Metadata, Record};
use serde::Serialize;
use std::collections::HashMap;
use std::marker::PhantomData;
use url::Url;

/// The `AuthenticationMethod` enum is used to specify the authentication method to use when
/// sending the log messages to the remote endpoint.
#[derive(Eq, PartialEq, Debug)]
pub enum AuthenticationMethod {
    /// Do not use any authentication when sending the log messages to the remote endpoint
    None,
    /// Use the HTTP Basic Auth when sending the log messages to the remote endpoint
    Basic,
}

/// The `Fenrir` struct implements the communication interface with a [Loki](https://grafana.com/oss/loki/)
/// instance.
///
/// To create a new instance of the `Fenrir` struct use the `FenrirBuilder` struct.
pub struct Fenrir<T: FenrirBackend> {
    backend: Box<dyn FenrirBackend + Send + Sync>,

    phantom: PhantomData<T>,
}

/// The `FenrirBuilder` struct is used to create a new instance of `Fenrir`using the builder pattern.
///
/// This should make it easier and more intuitive (at least I hope) to use the crate without referring
/// to the documentation all the time.
pub struct FenrirBuilder<T: FenrirBackend> {
    /// The loki `endpoint` which is used to send log information to
    endpoint: Url,
    /// The `authentication` method to use when sending the log messages to the remote endpoint
    authentication: AuthenticationMethod,
    /// The `credentials` to use to authenticate against the remote `endpoint`
    credentials: String,

    phantom: PhantomData<T>,
}

/// The `FenrirBackend` trait is used to specify the interfaces which are required for the communication
/// with the remote endpoint.
pub trait FenrirBackend {
    /// Sends a `Streams` object to the configured remote backend.
    fn send(&self, streams: &Streams) -> Result<(), String>;

    /// Get the configured `AuthenticationMethod` for the backend.
    fn authentication_method(&self) -> AuthenticationMethod;

    /// Get the configured credentials or `None` if no credentials are configured.
    fn credentials(&self) -> Option<String>;
}

impl<T: FenrirBackend> FenrirBuilder<T> {
    /// Set the endpoint which should be used for sending the log messages to.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    /// use fenrir_rs::noop::NoopBackend;
    ///
    /// let builder = Fenrir::<NoopBackend>::builder()
    ///     .endpoint(Url::parse("https://loki.example.com").unwrap());
    /// ```
    pub fn endpoint(mut self, endpoint: Url) -> FenrirBuilder<T> {
        self.endpoint = endpoint;
        self
    }

    /// Ensure our client uses the supplied credentials for authentication against the remote endpoint.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::{AuthenticationMethod, Fenrir};
    /// use fenrir_rs::noop::NoopBackend;
    ///
    /// let builder = Fenrir::<NoopBackend>::builder()
    ///     .with_authentication(AuthenticationMethod::Basic, "foo".to_string(), "bar".to_string());
    /// ```
    pub fn with_authentication(
        mut self,
        method: AuthenticationMethod,
        username: String,
        password: String,
    ) -> FenrirBuilder<T> {
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

    /// Create a new `Fenrir` instance with the parameters supplied to this struct before calling `build()`.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    /// use fenrir_rs::noop::NoopBackend;
    ///
    /// let fenrir = Fenrir::<NoopBackend>::builder().endpoint(Url::parse("https://loki.example.com").unwrap()).build();
    /// ```
    pub fn build(self) -> Fenrir<T> {
        use crate::noop::NoopBackend;

        Fenrir {
            backend: Box::new(NoopBackend {}),
            phantom: PhantomData,
        }
    }
}

impl<T: FenrirBackend> Fenrir<T> {
    /// Create a new `FenrirBuilder` with all required parameters.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::Fenrir;
    /// use fenrir_rs::noop::NoopBackend;
    ///
    /// let builder = Fenrir::<NoopBackend>::builder();
    /// ```
    pub fn builder() -> FenrirBuilder<T> {
        FenrirBuilder {
            endpoint: Url::parse("http://localhost:3100").unwrap(),
            authentication: AuthenticationMethod::None,
            credentials: "".to_string(),
            phantom: PhantomData,
        }
    }
}

#[derive(Serialize)]
pub struct Stream {
    pub stream: HashMap<String, String>,
    pub values: Vec<Vec<String>>,
}

#[derive(Serialize)]
pub struct Streams {
    pub streams: Vec<Stream>,
}

impl<T: FenrirBackend + Send + Sync> Log for Fenrir<T> {
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
        self.backend.send(&log_stream).unwrap();
    }

    fn flush(&self) {
        // TODO: implement the actual flushing
    }
}
