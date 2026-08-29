use super::sender::{
    checked_message, json_headers, json_string, DestinationKind, NotificationSender,
    OutboundRequest, SendError,
};
use crate::secrets::{SecretReference, SecretStore};

pub struct TelegramSender<'a> {
    pub bot_token: SecretReference,
    pub chat_id: String,
    pub secrets: &'a dyn SecretStore,
}
impl NotificationSender for TelegramSender<'_> {
    fn kind(&self) -> DestinationKind {
        DestinationKind::Telegram
    }
    fn request(&self, message: &str) -> Result<OutboundRequest, SendError> {
        checked_message(message)?;
        let token = self
            .secrets
            .get(&self.bot_token)
            .map_err(|_| SendError::Transport(super::sender::TransportError::Rejected))?;
        Ok(OutboundRequest {
            url: format!("https://api.telegram.org/bot{token}/sendMessage"),
            headers: json_headers(),
            body: format!(
                r#"{{"chat_id":{},"text":{},"disable_web_page_preview":true}}"#,
                json_string(&self.chat_id),
                json_string(message)
            ),
            timeout_ms: OutboundRequest::DEFAULT_TIMEOUT_MS,
        })
    }
}
