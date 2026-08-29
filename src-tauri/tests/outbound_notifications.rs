use std::{collections::BTreeMap, sync::Mutex};

use cookbench_desktop_lib::{
    notifications::{
        discord::DiscordSender,
        generic::GenericWebhookSender,
        lark::LarkSender,
        sender::{
            self, DestinationRateLimiter, NotificationSender, OutboundRequest, OutboundResponse,
            OutboundTransport, RetryClass, TransportError,
        },
        slack::SlackSender,
        telegram::TelegramSender,
    },
    secrets::{SecretError, SecretReference, SecretStore},
};

#[derive(Default)]
struct FakeSecrets(BTreeMap<String, String>);
impl FakeSecrets {
    fn with(reference: &SecretReference, value: &str) -> Self {
        Self(BTreeMap::from([(reference.redacted(), value.into())]))
    }
}
impl SecretStore for FakeSecrets {
    fn get(&self, reference: &SecretReference) -> Result<String, SecretError> {
        self.0
            .get(&reference.redacted())
            .cloned()
            .ok_or(SecretError::NotFound)
    }
    fn set(&self, _: &SecretReference, _: &str) -> Result<(), SecretError> {
        Ok(())
    }
    fn delete(&self, _: &SecretReference) -> Result<(), SecretError> {
        Ok(())
    }
}

struct FakeTransport {
    requests: Mutex<Vec<OutboundRequest>>,
    response: Result<OutboundResponse, TransportError>,
}
impl OutboundTransport for FakeTransport {
    fn post(&self, request: OutboundRequest) -> Result<OutboundResponse, TransportError> {
        self.requests.lock().unwrap().push(request);
        self.response.clone()
    }
}

fn reference(name: &str) -> SecretReference {
    SecretReference::new("Cookbench", name).unwrap()
}

#[test]
fn platform_senders_map_exact_outbound_payloads_without_exposing_credentials() {
    let telegram_ref = reference("telegram-token");
    let telegram_secrets = FakeSecrets::with(&telegram_ref, "synthetic-token");
    let telegram = TelegramSender {
        bot_token: telegram_ref.clone(),
        chat_id: "1234".into(),
        secrets: &telegram_secrets,
    };
    let telegram_request = telegram.request("Synthetic \"test\"").unwrap();
    assert_eq!(
        telegram_request.url,
        "https://api.telegram.org/botsynthetic-token/sendMessage"
    );
    assert_eq!(
        telegram_request.body,
        r#"{"chat_id":"1234","text":"Synthetic \"test\"","disable_web_page_preview":true}"#
    );
    assert_eq!(telegram_request.timeout_ms, 10_000);
    assert!(!format!("{:?}", telegram_ref).contains("synthetic-token"));

    let webhook = reference("webhook");
    let secrets = FakeSecrets::with(&webhook, "https://example.invalid/hook");
    assert_eq!(
        SlackSender {
            webhook: webhook.clone(),
            secrets: &secrets
        }
        .request("Synthetic test")
        .unwrap()
        .body,
        r#"{"text":"Synthetic test"}"#
    );
    assert_eq!(
        DiscordSender {
            webhook: webhook.clone(),
            secrets: &secrets
        }
        .request("Synthetic test")
        .unwrap()
        .body,
        r#"{"content":"Synthetic test"}"#
    );
    assert_eq!(
        LarkSender {
            webhook,
            secrets: &secrets
        }
        .request("Synthetic test")
        .unwrap()
        .body,
        r#"{"msg_type":"text","content":{"text":"Synthetic test"}}"#
    );
    let generic_webhook = reference("generic-webhook");
    let generic_secrets = FakeSecrets::with(&generic_webhook, "https://example.invalid/generic");
    assert_eq!(
        GenericWebhookSender {
            webhook: generic_webhook,
            secrets: &generic_secrets
        }
        .request("Synthetic test")
        .unwrap()
        .body,
        r#"{"text":"Synthetic test"}"#
    );
}

#[test]
fn outbound_only_transport_applies_timeout_retry_and_channel_isolation() {
    let timeout = Err(TransportError::Timeout);
    assert_eq!(sender::retry_class(&timeout), RetryClass::Retryable);
    assert_eq!(
        sender::retry_class(&Ok(OutboundResponse { status: 429 })),
        RetryClass::Retryable
    );
    assert_eq!(
        sender::retry_class(&Ok(OutboundResponse { status: 401 })),
        RetryClass::None
    );

    let webhook = reference("generic-webhook");
    let secrets = FakeSecrets::with(&webhook, "https://example.invalid/generic");
    let sender = GenericWebhookSender {
        webhook,
        secrets: &secrets,
    };
    let failed = FakeTransport {
        requests: Mutex::new(vec![]),
        response: Err(TransportError::Network),
    };
    assert!(sender.send(&failed, "Synthetic test").is_err());
    let healthy = FakeTransport {
        requests: Mutex::new(vec![]),
        response: Ok(OutboundResponse { status: 204 }),
    };
    assert!(sender.send(&healthy, "Synthetic test").is_ok());
    assert_eq!(failed.requests.lock().unwrap().len(), 1);
    assert_eq!(healthy.requests.lock().unwrap().len(), 1);

    let mut limiter = DestinationRateLimiter::default();
    assert!(limiter.allows("slack:one", 100));
    limiter.record("slack:one", 100);
    assert!(!limiter.allows("slack:one", 999));
    assert!(limiter.allows("discord:one", 999));
}

#[test]
fn payloads_are_bounded_and_synthetic_test_text_never_contains_a_secret() {
    let webhook = reference("generic-webhook");
    let secrets = FakeSecrets::with(&webhook, "https://example.invalid/generic");
    let sender = GenericWebhookSender {
        webhook,
        secrets: &secrets,
    };
    assert!(sender
        .request(&"x".repeat(OutboundRequest::MAX_MESSAGE_BYTES + 1))
        .is_err());
    let request = sender.request("Cookbench test notification").unwrap();
    assert_eq!(request.body, r#"{"text":"Cookbench test notification"}"#);
    assert!(!request.body.contains("secret://"));
}
