//! A module which contains the implementation for the [`FenrirBackend`] trait which ignores all
//! network requests.
use crate::{AuthenticationMethod, FenrirBackend};
use std::any::TypeId;

/// The [`NoopBackend`] is used by default and does ignore all logging messages.
pub(crate) struct NoopBackend;

impl FenrirBackend for NoopBackend {
    fn send(&self, _: Vec<u8>) -> Result<(), String> {
        Ok(())
    }

    fn internal_type(&self) -> TypeId {
        use std::any::Any;

        TypeId::of::<Self>().type_id()
    }

    fn authentication_method(&self) -> AuthenticationMethod {
        AuthenticationMethod::None
    }

    fn credentials(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::noop::NoopBackend;
    use crate::{AuthenticationMethod, Fenrir, NetworkingBackend, SerializationFormat};
    use std::any::{Any, TypeId};
    use url::Url;

    #[test]
    fn creating_a_noop_instance_without_credentials_works_correctly() {
        let result = Fenrir::builder()
            .endpoint(Url::parse("https://loki.example.com").unwrap())
            .network(NetworkingBackend::None)
            .format(SerializationFormat::Json)
            .build();
        assert_eq!(
            result.backend.authentication_method(),
            AuthenticationMethod::None
        );
        assert_eq!(result.backend.credentials(), None);
        assert_eq!(
            result.backend.internal_type(),
            TypeId::of::<NoopBackend>().type_id()
        );
    }

    #[test]
    fn creating_a_noop_instance_with_credentials_works_correctly() {
        let result = Fenrir::builder()
            .endpoint(Url::parse("https://loki.example.com").unwrap())
            .network(NetworkingBackend::None)
            .format(SerializationFormat::Json)
            .with_authentication(
                AuthenticationMethod::Basic,
                "username".to_string(),
                "password".to_string(),
            )
            .build();
        assert_eq!(
            result.backend.authentication_method(),
            AuthenticationMethod::None
        );
        assert_eq!(result.backend.credentials(), None);
        assert_eq!(
            result.backend.internal_type(),
            TypeId::of::<NoopBackend>().type_id()
        );
    }
}
