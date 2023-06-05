use crate::{AuthenticationMethod, FenrirBackend, SerializationFn, Streams};
use std::any::TypeId;

/// The `NoopBackend` is used by default and does ignore all logging messages.
pub(crate) struct NoopBackend;

impl FenrirBackend for NoopBackend {
    fn send(&self, _: &Streams, _: SerializationFn) -> Result<(), String> {
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
