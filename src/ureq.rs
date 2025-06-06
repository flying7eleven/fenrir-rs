//! A module which contains the implementation for the [`FenrirBackend`] trait which uses the `ureq`
//! crate for network communication.
use crate::{AuthenticationMethod, FenrirBackend};
use std::any::TypeId;
use ureq::Agent;
use url::Url;

/// A [`FenrirBackend`] implementation which uses the [ureq](https://crates.io/crates/ureq) crate to
/// send logging messages to a Loki endpoint.
pub(crate) struct UreqBackend {
    /// The loki endpoint which is used to send log information to
    pub(crate) endpoint: Url,
    /// The authentication method to use when sending the log messages to the remote [`UreqBackend::endpoint`]
    pub(crate) authentication: AuthenticationMethod,
    /// The credentials to use to authenticate against the remote [`UreqBackend::endpoint`]
    pub(crate) credentials: String,
}

impl FenrirBackend for UreqBackend {
    fn send(&self, serialized_streams: Vec<u8>) -> Result<(), String> {
        use std::time::Duration;

        let agent_config = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build();

        let post_url = self
            .endpoint
            .clone()
            .join("/loki/api/v1/push")
            .map_err(|e| e.to_string())?;
        let agent = agent_config.new_agent();
        let mut request = agent.post(post_url.as_str());
        request = request.header("Content-Type", "application/json; charset=utf-8");
        match self.authentication {
            AuthenticationMethod::None => {}
            AuthenticationMethod::Basic => {
                request = request.header(
                    "Authorization",
                    format!("Basic {}", self.credentials).as_str(),
                );
            }
        }

        request
            .send(&serialized_streams)
            .map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests {
    use crate::ureq::UreqBackend;
    use crate::{AuthenticationMethod, Fenrir, NetworkingBackend, SerializationFormat};
    use std::any::{Any, TypeId};
    use url::Url;

    #[test]
    fn creating_a_ureq_instance_without_credentials_works_correctly() {
        let result = Fenrir::builder()
            .endpoint(Url::parse("https://loki.example.com").unwrap())
            .network(NetworkingBackend::Ureq)
            .format(SerializationFormat::Json)
            .build();
        assert_eq!(
            result.backend.authentication_method(),
            AuthenticationMethod::None
        );
        assert_eq!(result.backend.credentials(), None);
        assert_eq!(
            result.backend.internal_type(),
            TypeId::of::<UreqBackend>().type_id()
        );
    }

    #[test]
    fn creating_a_ureq_instance_with_credentials_works_correctly() {
        let result = Fenrir::builder()
            .endpoint(Url::parse("https://loki.example.com").unwrap())
            .network(NetworkingBackend::Ureq)
            .format(SerializationFormat::Json)
            .with_authentication(
                AuthenticationMethod::Basic,
                "username".to_string(),
                "password".to_string(),
            )
            .build();
        assert_eq!(
            result.backend.authentication_method(),
            AuthenticationMethod::Basic
        );
        assert_eq!(
            result.backend.credentials(),
            Some("dXNlcm5hbWU6cGFzc3dvcmQ=".to_string())
        );
        assert_eq!(
            result.backend.internal_type(),
            TypeId::of::<UreqBackend>().type_id()
        );
    }
}
