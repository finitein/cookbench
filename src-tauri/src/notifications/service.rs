//! Outbound notification orchestration. This module only builds POST requests
//! after a stove transition; it never accepts messages or controls a harness.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, SystemTime};

use cookbench_core::notifications::{
    evaluate, BoundedQueue, DestinationId, NotificationContext, NotificationSettings, QueueItem,
    Template, TemplateContext,
};

use crate::secrets::{SecretReference, SecretStore};

use super::{
    discord::DiscordSender,
    generic::GenericWebhookSender,
    lark::LarkSender,
    sender::{
        retry_class, DestinationKind, NotificationSender, OutboundTransport, RetryClass, SendError,
    },
    slack::SlackSender,
    telegram::TelegramSender,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DestinationConfiguration {
    pub id: DestinationId,
    pub kind: DestinationKind,
    pub enabled: bool,
    pub secret: SecretReference,
    /// A Telegram chat ID is public destination metadata. Other senders ignore it.
    pub recipient: Option<String>,
}

impl DestinationConfiguration {
    pub fn redacted_id(&self) -> String {
        self.id.as_str().to_owned()
    }
}

pub struct NotificationService<T: OutboundTransport, S: SecretStore> {
    transport: T,
    secrets: S,
    settings: Mutex<NotificationSettings>,
    destinations: Mutex<Vec<DestinationConfiguration>>,
    queue: Mutex<BoundedQueue>,
    flushing: AtomicBool,
    flush_requested: AtomicBool,
}

impl<T: OutboundTransport, S: SecretStore> NotificationService<T, S> {
    pub fn new(transport: T, secrets: S) -> Self {
        Self {
            transport,
            secrets,
            settings: Mutex::new(NotificationSettings::default()),
            destinations: Mutex::new(Vec::new()),
            queue: Mutex::new(BoundedQueue::new(128, 60_000, 3)),
            flushing: AtomicBool::new(false),
            flush_requested: AtomicBool::new(false),
        }
    }

    pub fn configure(
        &self,
        settings: NotificationSettings,
        destinations: Vec<DestinationConfiguration>,
    ) {
        *self
            .settings
            .lock()
            .expect("notification settings lock poisoned") = settings;
        *self
            .destinations
            .lock()
            .expect("notification destinations lock poisoned") = destinations;
    }

    pub fn set_secret(
        &self,
        reference: &SecretReference,
        value: &str,
    ) -> Result<(), crate::secrets::SecretError> {
        self.secrets.set(reference, value)
    }

    /// Evaluates a validated transition for every enabled destination. A queue
    /// admission is best effort and can never affect the stove state reducer.
    pub fn enqueue_transition(&self, context: &NotificationContext, now_ms: u64) {
        let settings = self
            .settings
            .lock()
            .expect("notification settings lock poisoned")
            .clone();
        let destinations = self
            .destinations
            .lock()
            .expect("notification destinations lock poisoned")
            .clone();
        let template =
            Template::parse("{project}: {state} {activity}").expect("built-in template is valid");
        let mut queue = self.queue.lock().expect("notification queue lock poisoned");
        for destination in destinations
            .into_iter()
            .filter(|destination| destination.enabled)
        {
            let mut scoped = context.clone();
            scoped.destination = destination.id;
            let decision = evaluate(&settings, &scoped);
            if !decision.should_notify {
                continue;
            }
            let rendered = decision
                .template
                .as_ref()
                .unwrap_or(&template)
                .render(&TemplateContext::from(&scoped), 4_096);
            if let Ok(message) = rendered {
                let _ = queue.push(QueueItem::new(scoped, message, now_ms));
            }
        }
    }

    /// Sends one synthetic message to exactly one configured destination.
    pub fn send_test(&self, destination_id: &DestinationId) -> Result<(), SendError> {
        let destination = self
            .destinations
            .lock()
            .expect("notification destinations lock poisoned")
            .iter()
            .find(|candidate| &candidate.id == destination_id && candidate.enabled)
            .cloned()
            .ok_or(SendError::RateLimited)?;
        self.send(&destination, "Cookbench test notification")
    }

    /// Attempts one queued outbound delivery. The caller may invoke this on a
    /// bounded timer; no receive loop, listener, or remote control exists here.
    pub fn flush_one(&self, now_ms: u64) -> Option<Result<(), RetryClass>> {
        let item = self
            .queue
            .lock()
            .expect("notification queue lock poisoned")
            .pop_ready(now_ms)?;
        let destination = self
            .destinations
            .lock()
            .expect("notification destinations lock poisoned")
            .iter()
            .find(|candidate| candidate.id == item.context.destination && candidate.enabled)
            .cloned()?;
        match self.send(&destination, &item.message) {
            Ok(()) => Some(Ok(())),
            Err(error) => {
                let class = retry_class(&send_error_result(&error));
                if class == RetryClass::Retryable {
                    let _ = self
                        .queue
                        .lock()
                        .expect("notification queue lock poisoned")
                        .requeue_failed(item, now_ms);
                }
                Some(Err(class))
            }
        }
    }

    pub fn flush_bounded(&self, now_ms: u64, maximum: usize) {
        for _ in 0..maximum {
            if self.flush_one(now_ms).is_none() {
                break;
            }
        }
    }

    pub fn request_flush(self: &Arc<Self>, now_ms: u64)
    where
        T: 'static,
        S: 'static,
    {
        self.flush_requested.store(true, Ordering::Release);
        if self
            .flushing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let service = Arc::clone(self);
        std::thread::spawn(move || loop {
            service.flush_requested.store(false, Ordering::Release);
            let current = wall_clock_ms().max(now_ms);
            service.flush_bounded(current, 128);
            let next_ready = service
                .queue
                .lock()
                .expect("notification queue lock poisoned")
                .next_ready_at_ms();
            if let Some(next_ready) = next_ready {
                // Wake at least once per second so a newly queued critical
                // transition is not hidden behind an unrelated retry delay.
                let delay = next_ready.saturating_sub(current).min(1_000);
                if delay > 0 {
                    std::thread::sleep(Duration::from_millis(delay));
                }
                continue;
            }
            service.flushing.store(false, Ordering::Release);
            if !service.flush_requested.swap(false, Ordering::AcqRel) {
                break;
            }
            if service
                .flushing
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                break;
            }
        });
    }

    fn send(&self, destination: &DestinationConfiguration, message: &str) -> Result<(), SendError> {
        match destination.kind {
            DestinationKind::Telegram => TelegramSender {
                bot_token: destination.secret.clone(),
                chat_id: destination.recipient.clone().unwrap_or_default(),
                secrets: &self.secrets,
            }
            .send(&self.transport, message),
            DestinationKind::Slack => SlackSender {
                webhook: destination.secret.clone(),
                secrets: &self.secrets,
            }
            .send(&self.transport, message),
            DestinationKind::Discord => DiscordSender {
                webhook: destination.secret.clone(),
                secrets: &self.secrets,
            }
            .send(&self.transport, message),
            DestinationKind::Lark => LarkSender {
                webhook: destination.secret.clone(),
                secrets: &self.secrets,
            }
            .send(&self.transport, message),
            DestinationKind::Generic => GenericWebhookSender {
                webhook: destination.secret.clone(),
                secrets: &self.secrets,
            }
            .send(&self.transport, message),
        }
    }
}

fn wall_clock_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn send_error_result(
    error: &SendError,
) -> Result<super::sender::OutboundResponse, super::sender::TransportError> {
    match error {
        SendError::Transport(error) => Err(error.clone()),
        SendError::HttpStatus(status) => Ok(super::sender::OutboundResponse { status: *status }),
        _ => Err(super::sender::TransportError::Rejected),
    }
}
