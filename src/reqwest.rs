//! A module which contains the implementation for the [`FenrirBackend`] trait which uses the `reqwest`
//! crate for network communication.

use crate::{AuthenticationMethod, FenrirBackend};
use reqwest::Client;
use std::any::TypeId;
use url::Url;

/// A [`FenrirBackend`] implementation which uses the [reqwest](https://crates.io/crates/reqwest) crate to
/// send logging messages to a Loki endpoint.
pub(crate) struct ReqwestBackend {
    /// The loki endpoint which is used to send log information to
    pub(crate) endpoint: Url,
    /// The authentication method to use when sending the log messages to the remote [`UreqBackend::endpoint`]
    pub(crate) authentication: AuthenticationMethod,
    /// The credentials to use to authenticate against the remote [`UreqBackend::endpoint`]
    pub(crate) credentials: String,
    /// Internal client
    pub(crate) client: Client,
    /// Runtime handle
    pub(crate) runtime_handle: tokio::runtime::Handle,
}

impl FenrirBackend for ReqwestBackend {
    fn send(&self, serialized_streams: Vec<u8>) -> Result<(), String> {
        let post_url: Url = self
            .endpoint
            .clone()
            .join("/loki/api/v1/push")
            .map_err(|e| e.to_string())?;
        let mut builder = self
            .client
            .post(post_url)
            .header("Content-Type", "application/json; charset=utf-8");
        if let AuthenticationMethod::Basic = self.authentication {
            builder = builder.header(
                "Authorization",
                format!("Basic {}", self.credentials).as_str(),
            );
        }
        builder = builder.body(serialized_streams);
        let runtime_handle = self.runtime_handle.clone();
        runtime_handle.spawn(async move {
            // retry 3 times if failed with a non-400 error
            let mut retry_count = 0;
            loop {
                let b2 = builder.try_clone().expect("should be able to clone");
                let res = builder.send().await;
                match res {
                    Ok(_) => {
                        break;
                    }
                    Err(e) => {
                        if e.status().map_or(false, |x| x.is_client_error()) || retry_count >= 3 {
                            log::error!("Failed to send logs to Loki: {}", e);
                            break;
                        }
                        retry_count += 1;
                    }
                }
                builder = b2;
            }
        });

        Ok(())
    }

    fn internal_type(&self) -> TypeId {
        use std::any::Any;

        TypeId::of::<Self>().type_id()
    }

    fn authentication_method(&self) -> AuthenticationMethod {
        self.authentication.clone()
    }

    fn credentials(&self) -> Option<String> {
        if self.credentials.len() > 0 {
            return Some(self.credentials.clone());
        }
        None
    }
}
