import { useEffect, useState } from "react";

import type { NotificationDestination } from "./NotificationSettings";
import {
  configureNotificationDestination,
  configureLocalNotificationSettings,
  getLocalNotificationSettings,
  getNotificationSettings,
  sendTestNotification,
  testLocalNotification,
  type LocalNotificationChannel,
  type LocalNotificationSettingsWire,
  type NotificationDestinationWire,
  type NotificationEvent,
} from "./service";
import "./notification-settings.css";
import { RemoteSourcesPanel } from "../remote/RemoteSourcesPanel";
import { DisplaySettingsPanel } from "../display/DisplaySettingsPanel";
import { SourcesStatusPanel } from "../sources/SourcesStatusPanel";
import { HookHealthPanel } from "../hooks/HookHealthPanel";
import { ArchiveSettingsPanel } from "../archive/ArchiveSettingsPanel";

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

const LOCAL_CHANNELS: Array<{ id: LocalNotificationChannel; label: string }> = [
  { id: "sound", label: "Sound" },
  { id: "systemBanner", label: "System notification" },
  { id: "barFlash", label: "Flash Stove" },
  { id: "systemAttention", label: "Request attention" },
];

const DEFAULT_LOCAL_NOTIFICATION_SETTINGS: LocalNotificationSettingsWire = {
  sound: true,
  systemBanner: false,
  barFlash: false,
  systemAttention: false,
  events: ["needsHuman", "cooked", "failed", "disconnected"],
};

export function NotificationSettingsPanel() {
  const [tab, setTab] = useState<"general" | "archive">("general");
  const [destinations, setDestinations] = useState<NotificationDestinationWire[]>([]);
  const [localSettings, setLocalSettings] = useState<LocalNotificationSettingsWire>(
    DEFAULT_LOCAL_NOTIFICATION_SETTINGS,
  );
  const [secrets, setSecrets] = useState<Partial<Record<NotificationDestination, string>>>({});
  const [busy, setBusy] = useState<NotificationDestination | null>(null);
  const [localBusy, setLocalBusy] = useState<"save" | LocalNotificationChannel | null>(null);
  const [status, setStatus] = useState("");

  useEffect(() => {
    void getNotificationSettings().then(setDestinations).catch(() => {
      setStatus("Notification settings are unavailable.");
    });
    void getLocalNotificationSettings().then(setLocalSettings).catch(() => {
      setStatus("Local alert settings are unavailable.");
    });
  }, []);

  useEffect(() => {
    if (!status) return undefined;
    const timeout = window.setTimeout(() => setStatus(""), 20_000);
    return () => window.clearTimeout(timeout);
  }, [status]);

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

  const updateLocal = (change: Partial<LocalNotificationSettingsWire>) => {
    setLocalSettings((current) => ({ ...current, ...change }));
  };

  const saveLocal = async () => {
    setLocalBusy("save");
    setStatus("");
    try {
      setLocalSettings(await configureLocalNotificationSettings(localSettings));
      setStatus("Local alerts saved.");
    } catch {
      setStatus("Local alerts could not be saved.");
    } finally {
      setLocalBusy(null);
    }
  };

  const testLocal = async (channel: LocalNotificationChannel) => {
    setLocalBusy(channel);
    setStatus("");
    try {
      const result = await testLocalNotification(channel);
      const label = LOCAL_CHANNELS.find((item) => item.id === channel)?.label ?? "Local alert";
      setStatus(
        result === "delivered"
          ? `${label} test sent.`
          : result === "permissionDenied"
            ? `${label} needs system notification permission.`
            : `${label} is unavailable on this system.`,
      );
    } catch {
      setStatus("Local alert test failed.");
    } finally {
      setLocalBusy(null);
    }
  };

  return (
    <main className="notification-settings" aria-label="Cookbench settings">
      <div className="notification-settings__surface">
        <header className="notification-settings__masthead">
          <div>
            <p>Cookbench</p>
            <h1>Settings</h1>
          </div>
        </header>
        <div className="notification-settings__tabs" role="tablist" aria-label="Settings sections">
          <button type="button" role="tab" aria-selected={tab === "general"} onClick={() => setTab("general")}>General</button>
          <button type="button" role="tab" aria-selected={tab === "archive"} onClick={() => setTab("archive")}>Archive</button>
        </div>
        {tab === "archive" ? <ArchiveSettingsPanel /> : <>
        <DisplaySettingsPanel />
        <section aria-labelledby="local-alerts-title">
          <div className="notification-settings__section-heading">
            <h2 id="local-alerts-title">Local alerts</h2>
          </div>
          <div className="notification-settings__local-alerts">
            {LOCAL_CHANNELS.map((channel) => (
              <div className="notification-settings__local-channel" key={channel.id}>
                <label className="notification-settings__toggle">
                  <input
                    type="checkbox"
                    checked={localSettings[channel.id]}
                    onChange={(event) => updateLocal({ [channel.id]: event.target.checked })}
                  />
                  <span>{channel.label}</span>
                </label>
                <button
                  type="button"
                  aria-label={`Test ${channel.label}`}
                  disabled={localBusy !== null}
                  onClick={() => void testLocal(channel.id)}
                >
                  Test
                </button>
              </div>
            ))}
            <fieldset>
              <legend>States</legend>
              {EVENTS.map((event) => (
                <label key={event.id}>
                  <input
                    type="checkbox"
                    checked={localSettings.events.includes(event.id)}
                    onChange={(input) => updateLocal({
                      events: input.target.checked
                        ? [...localSettings.events, event.id]
                        : localSettings.events.filter((candidate) => candidate !== event.id),
                    })}
                  />
                  {event.label}
                </label>
              ))}
            </fieldset>
            <div className="notification-settings__actions">
              <button type="button" disabled={localBusy !== null} onClick={() => void saveLocal()}>Save</button>
            </div>
          </div>
        </section>
        <section aria-labelledby="notification-settings-title">
          <div className="notification-settings__section-heading">
            <h2 id="notification-settings-title">Notifications</h2>
          </div>
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
        <SourcesStatusPanel />
        <HookHealthPanel />
        <RemoteSourcesPanel />
        </>}
      </div>
    </main>
  );
}
