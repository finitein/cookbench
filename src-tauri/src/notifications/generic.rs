use super::sender::{
    checked_message, checked_webhook_url, json_headers, json_string, DestinationKind,
    NotificationSender, OutboundRequest, SendError,
};
use crate::secrets::{SecretReference, SecretStore};

/// Generic webhooks are explicit destinations. Their URL is configuration, not
/// a received callback endpoint, and delivery is always a single POST.
pub struct GenericWebhookSender<'a> {
    pub webhook: SecretReference,
    pub secrets: &'a dyn SecretStore,
}
impl NotificationSender for GenericWebhookSender<'_> {
    fn kind(&self) -> DestinationKind {
        DestinationKind::Generic
    }
    fn request(&self, message: &str) -> Result<OutboundRequest, SendError> {
        checked_message(message)?;
        let url = self
            .secrets
            .get(&self.webhook)
            .map_err(|_| SendError::Transport(super::sender::TransportError::Rejected))?;
        checked_webhook_url(&url)?;
        Ok(OutboundRequest {
            url,
            headers: json_headers(),
            body: format!(r#"{{"text":{}}}"#, json_string(message)),
            timeout_ms: OutboundRequest::DEFAULT_TIMEOUT_MS,
        })
    }
}
