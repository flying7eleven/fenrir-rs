#![doc = include_str!("../README.md")]

use log::{Log, Metadata, Record};
use serde::Serialize;
use std::collections::HashMap;
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
pub struct Fenrir {
    /// The loki `endpoint` which is used to send log information to
    endpoint: Url,
    /// The `authentication` method to use when sending the log messages to the remote endpoint
    authentication: AuthenticationMethod,
    /// The `credentials` to use to authenticate against the remote `endpoint`
    credentials: String,
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
    /// The `credentials` to use to authenticate against the remote `endpoint`
    credentials: String,
}

impl FenrirBuilder {
    /// Create a new `FenrirBuilder` with all required parameters.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::FenrirBuilder;
    ///
    /// let builder = FenrirBuilder::new(Url::parse("https://loki.example.com").unwrap());
    /// ```
    pub fn new(endpoint: Url) -> FenrirBuilder {
        FenrirBuilder {
            endpoint,
            authentication: AuthenticationMethod::None,
            credentials: "".to_string(),
        }
    }

    /// Ensure our client uses the supplied credentials for authentication against the remote endpoint.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::{AuthenticationMethod, FenrirBuilder};
    ///
    /// let builder = FenrirBuilder::new(Url::parse("https://loki.example.com").unwrap())
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

    /// Create a new `Fenrir` instance with the parameters supplied to this struct before calling `build()`.
    ///
    /// # Example
    /// ```
    /// use url::Url;
    /// use fenrir_rs::FenrirBuilder;
    ///
    /// let fenrir = FenrirBuilder::new(Url::parse("https://loki.example.com").unwrap()).build();
    /// ```
    pub fn build(self) -> Fenrir {
        Fenrir {
            endpoint: self.endpoint,
            authentication: self.authentication,
            credentials: self.credentials,
        }
    }
}

#[derive(Serialize)]
struct Stream {
    pub stream: HashMap<String, String>,
    pub values: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct Streams {
    pub streams: Vec<Stream>,
}

impl Log for Fenrir {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true // TODO: use the metadata object to decide if we should be enabled or not
    }

    fn log(&self, record: &Record) {
        use serde_json::to_string;
        use std::time::Duration;
        use std::time::{SystemTime, UNIX_EPOCH};
        use ureq::AgentBuilder;

        let module = record.module_path().unwrap_or("");
        if module.starts_with("ureq") || !self.enabled(record.metadata()) {
            return;
        }

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
        let log_stream_text = to_string(&log_stream).unwrap();

        let post_url = self.endpoint.clone().join("/loki/api/v1/push").unwrap();
        let agent = AgentBuilder::new().timeout(Duration::from_secs(10)).build();
        let mut request = agent.request_url("POST", &post_url);
        request = request.set("Content-Type", "application/json; charset=utf-8");
        match self.authentication {
            AuthenticationMethod::None => {}
            AuthenticationMethod::Basic => {
                request = request.set(
                    "Authorization",
                    format!("Basic {}", self.credentials).as_str(),
                );
            }
        }
        let _ = request.send_string(log_stream_text.as_str()).unwrap();
    }

    fn flush(&self) {
        // TODO: implement the actual flushing
    }
}

#[cfg(test)]
mod tests {
    use crate::{AuthenticationMethod, FenrirBuilder};
    use url::Url;

    #[test]
    fn creating_an_instance_without_credentials_works_correctly() {
        let result = FenrirBuilder::new(Url::parse("https://loki.example.com").unwrap()).build();
        assert_eq!(result.authentication, AuthenticationMethod::None);
        assert_eq!(result.credentials, "".to_string());
    }

    #[test]
    fn creating_an_instance_with_credentials_works_correctly() {
        let result = FenrirBuilder::new(Url::parse("https://loki.example.com").unwrap())
            .with_authentication(
                AuthenticationMethod::Basic,
                "username".to_string(),
                "password".to_string(),
            )
            .build();
        assert_eq!(result.authentication, AuthenticationMethod::Basic);
        assert_eq!(result.credentials, "dXNlcm5hbWU6cGFzc3dvcmQ=".to_string());
    }
}
