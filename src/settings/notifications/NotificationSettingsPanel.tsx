import { useEffect, useState } from "react";

import type { NotificationDestination } from "./NotificationSettings";
import {
  configureNotificationDestination,
  getNotificationSettings,
  sendTestNotification,
  type NotificationDestinationWire,
  type NotificationEvent,
} from "./service";
import "./notification-settings.css";
import { RemoteSourcesPanel } from "../remote/RemoteSourcesPanel";

const LABELS: Record<NotificationDestination, string> = {
  telegram: "Telegram",
  slack: "Slack",
  discord: "Discord",
  lark: "Lark / Feishu",
  generic: "Generic Webhook",
};

const EVENTS: Array<{ id: NotificationEvent; label: string }> = [
  { id: "sessionAppeared", label: "Session Appeared" },
  { id: "cookingStarted", label: "Cooking Started" },
  { id: "phaseChanged", label: "Phase Changed" },
  { id: "needsHuman", label: "Needs Human" },
  { id: "progressMilestone", label: "Progress Milestone" },
  { id: "cooked", label: "Cooked" },
  { id: "failed", label: "Failed" },
  { id: "disconnected", label: "Disconnected" },
  { id: "connectionRestored", label: "Connection Restored" },
  { id: "stoveCleared", label: "Stove Cleared" },
];

export function NotificationSettingsPanel() {
  const [destinations, setDestinations] = useState<NotificationDestinationWire[]>([]);
  const [secrets, setSecrets] = useState<Partial<Record<NotificationDestination, string>>>({});
  const [busy, setBusy] = useState<NotificationDestination | null>(null);
  const [status, setStatus] = useState("");

  useEffect(() => {
    void getNotificationSettings().then(setDestinations).catch(() => {
      setStatus("Notification settings are unavailable.");
    });
  }, []);

  const update = (
    destination: NotificationDestination,
    change: Partial<NotificationDestinationWire>,
  ) => {
    setDestinations((current) => current.map((item) => (
      item.destination === destination ? { ...item, ...change } : item
    )));
  };

  const save = async (item: NotificationDestinationWire) => {
    setBusy(item.destination);
    setStatus("");
    try {
      const next = await configureNotificationDestination({
        destination: item.destination,
        enabled: item.enabled,
        secret: secrets[item.destination] || null,
        recipient: item.recipient,
        events: item.events,
        template: item.template,
      });
      setDestinations(next);
      setSecrets((current) => ({ ...current, [item.destination]: "" }));
      setStatus(`${LABELS[item.destination]} saved.`);
    } catch {
      setStatus(`${LABELS[item.destination]} could not be saved.`);
    } finally {
      setBusy(null);
    }
  };

  const test = async (item: NotificationDestinationWire) => {
    setBusy(item.destination);
    setStatus("");
    try {
      await sendTestNotification(item.destination);
      setStatus(`${LABELS[item.destination]} test sent.`);
    } catch {
      setStatus(`${LABELS[item.destination]} test failed.`);
    } finally {
      setBusy(null);
    }
  };

  return (
    <main className="notification-settings" aria-label="Cookbench settings">
      <header>
        <h1>Settings</h1>
      </header>
      <section aria-labelledby="notification-settings-title">
      <h2 id="notification-settings-title">Notifications</h2>
      <div className="notification-settings__destinations">
        {destinations.map((item) => (
          <section key={item.destination} aria-label={LABELS[item.destination]}>
            <div className="notification-settings__heading">
              <strong>{LABELS[item.destination]}</strong>
              <label className="notification-settings__toggle">
                <input
                  type="checkbox"
                  checked={item.enabled}
                  onChange={(event) => update(item.destination, { enabled: event.target.checked })}
                />
                <span>Enabled</span>
              </label>
            </div>
            <div className="notification-settings__fields">
              <label>
                <span>{item.destination === "telegram" ? "Bot token" : "Webhook URL"}</span>
                <input
                  type="password"
                  value={secrets[item.destination] ?? ""}
                  autoComplete="off"
                  placeholder={item.configured ? "Stored in system credentials" : "Not configured"}
                  onChange={(event) => setSecrets((current) => ({
                    ...current,
                    [item.destination]: event.target.value,
                  }))}
                />
              </label>
              {item.destination === "telegram" ? (
                <label>
                  <span>Chat ID</span>
                  <input
                    value={item.recipient ?? ""}
                    onChange={(event) => update(item.destination, { recipient: event.target.value })}
                  />
                </label>
              ) : null}
            </div>
            <fieldset>
              <legend>States</legend>
              {EVENTS.map((event) => (
                <label key={event.id}>
                  <input
                    type="checkbox"
                    checked={item.events.includes(event.id)}
                    onChange={(input) => update(item.destination, {
                      events: input.target.checked
                        ? [...item.events, event.id]
                        : item.events.filter((candidate) => candidate !== event.id),
                    })}
                  />
                  {event.label}
                </label>
              ))}
            </fieldset>
            <label className="notification-settings__template">
              <span>Message template</span>
              <input
                value={item.template ?? ""}
                maxLength={1024}
                placeholder="{project}: {state} {activity}"
                onChange={(event) => update(item.destination, {
                  template: event.target.value || null,
                })}
              />
            </label>
            <div className="notification-settings__actions">
              <button type="button" disabled={busy === item.destination} onClick={() => void save(item)}>Save</button>
              <button
                type="button"
                disabled={!item.enabled || !item.configured || busy === item.destination}
                onClick={() => void test(item)}
              >
                Test
              </button>
            </div>
          </section>
        ))}
      </div>
      <output role="status" aria-live="polite">{status}</output>
      </section>
      <RemoteSourcesPanel />
    </main>
  );
}
