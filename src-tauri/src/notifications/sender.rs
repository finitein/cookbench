use std::{collections::BTreeMap, fmt, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DestinationKind {
    Telegram,
    Slack,
    Discord,
    Lark,
    Generic,
}

#[derive(Clone, Eq, PartialEq)]
pub struct OutboundRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub timeout_ms: u64,
}

impl fmt::Debug for OutboundRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutboundRequest")
            .field("url", &"[redacted]")
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

impl OutboundRequest {
    pub const MAX_MESSAGE_BYTES: usize = 4_096;
    pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutboundResponse {
    pub status: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportError {
    Timeout,
    Network,
    Rejected,
}

/// Outbound HTTP only. This trait intentionally has no listener, poll, bind,
/// receive-message, or agent-control operation.
pub trait OutboundTransport: Send + Sync {
    fn post(&self, request: OutboundRequest) -> Result<OutboundResponse, TransportError>;
}

/// Concrete outbound-only HTTP transport. Redirects are disabled so a
/// destination cannot forward credential-bearing requests to another host.
#[derive(Clone, Debug)]
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, TransportError> {
        reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map(|client| Self { client })
            .map_err(|_| TransportError::Network)
    }
}

impl OutboundTransport for ReqwestTransport {
    fn post(&self, request: OutboundRequest) -> Result<OutboundResponse, TransportError> {
        let mut builder = self
            .client
            .post(&request.url)
            .timeout(Duration::from_millis(request.timeout_ms))
            .body(request.body);
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        builder
            .send()
            .map(|response| OutboundResponse {
                status: response.status().as_u16(),
            })
            .map_err(|error| {
                if error.is_timeout() {
                    TransportError::Timeout
                } else if error.is_builder() {
                    TransportError::Rejected
                } else {
                    TransportError::Network
                }
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetryClass {
    None,
    Retryable,
}

pub fn retry_class(result: &Result<OutboundResponse, TransportError>) -> RetryClass {
    match result {
        Err(TransportError::Timeout | TransportError::Network) => RetryClass::Retryable,
        Err(TransportError::Rejected) => RetryClass::None,
        Ok(response)
            if response.status == 408 || response.status == 429 || response.status >= 500 =>
        {
            RetryClass::Retryable
        }
        Ok(_) => RetryClass::None,
    }
}

pub fn is_success(response: &OutboundResponse) -> bool {
    (200..300).contains(&response.status)
}

#[derive(Default)]
pub struct DestinationRateLimiter {
    last_sent_ms: BTreeMap<String, u64>,
}

impl DestinationRateLimiter {
    pub const MIN_INTERVAL_MS: u64 = 1_000;

    pub fn allows(&self, destination: &str, now_ms: u64) -> bool {
        self.last_sent_ms
            .get(destination)
            .is_none_or(|last| now_ms.saturating_sub(*last) >= Self::MIN_INTERVAL_MS)
    }
    pub fn record(&mut self, destination: impl Into<String>, now_ms: u64) {
        self.last_sent_ms.insert(destination.into(), now_ms);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SendError {
    RateLimited,
    Transport(TransportError),
    HttpStatus(u16),
    MessageTooLong,
    InvalidWebhookUrl,
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimited => f.write_str("notification destination is rate limited"),
            Self::Transport(_) => f.write_str("outbound notification transport failed"),
            Self::HttpStatus(status) => {
                write!(f, "outbound notification rejected with HTTP {status}")
            }
            Self::MessageTooLong => f.write_str("notification message exceeds the outbound limit"),
            Self::InvalidWebhookUrl => {
                f.write_str("notification destination must use a bounded HTTPS URL")
            }
        }
    }
}
impl std::error::Error for SendError {}

pub trait NotificationSender: Send + Sync {
    fn kind(&self) -> DestinationKind;
    fn request(&self, message: &str) -> Result<OutboundRequest, SendError>;

    fn send(&self, transport: &impl OutboundTransport, message: &str) -> Result<(), SendError> {
        let response = transport
            .post(self.request(message)?)
            .map_err(SendError::Transport)?;
        if is_success(&response) {
            Ok(())
        } else {
            Err(SendError::HttpStatus(response.status))
        }
    }
}

pub fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub fn checked_message(message: &str) -> Result<(), SendError> {
    if message.len() > OutboundRequest::MAX_MESSAGE_BYTES {
        Err(SendError::MessageTooLong)
    } else {
        Ok(())
    }
}

pub fn checked_webhook_url(url: &str) -> Result<(), SendError> {
    if url.starts_with("https://") && url.len() <= 2_048 {
        Ok(())
    } else {
        Err(SendError::InvalidWebhookUrl)
    }
}

pub fn json_headers() -> Vec<(String, String)> {
    vec![("content-type".to_owned(), "application/json".to_owned())]
}
