use crate::{AuthenticationMethod, FenrirBackend, Streams};
use url::Url;

pub struct UreqBackend {
    /// The loki `endpoint` which is used to send log information to
    endpoint: Url,
    /// The `authentication` method to use when sending the log messages to the remote endpoint
    authentication: AuthenticationMethod,
    /// The `credentials` to use to authenticate against the remote `endpoint`
    credentials: String,
}

#[cfg(not(all(feature = "json")))]
#[inline]
pub fn to_string<T>(_: &T) -> Result<String, ()>
where
    T: ?Sized + serde::Serialize,
{
    Ok("".to_string())
}

impl FenrirBackend for UreqBackend {
    fn send(&self, streams: &Streams) -> Result<(), String> {
        #[cfg(feature = "json")]
        use serde_json::to_string;
        use std::time::Duration;
        use ureq::AgentBuilder;

        let log_stream_text = to_string(streams).unwrap();

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

        Ok(())
    }
}