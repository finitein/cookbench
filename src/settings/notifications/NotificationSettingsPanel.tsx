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
import { useI18n, type TranslationKey } from "../../i18n/i18n";

const LABELS: Record<NotificationDestination, string> = {
  telegram: "Telegram",
  slack: "Slack",
  discord: "Discord",
  lark: "Lark / Feishu",
  generic: "Generic Webhook",
};

const EVENTS: NotificationEvent[] = ["sessionAppeared", "cookingStarted", "phaseChanged", "needsHuman", "progressMilestone", "cooked", "failed", "disconnected", "connectionRestored", "stoveCleared"];
const EVENT_KEYS: Record<NotificationEvent, TranslationKey> = {
  sessionAppeared: "notifications.sessionAppeared", cookingStarted: "notifications.cookingStarted",
  phaseChanged: "notifications.phaseChanged", needsHuman: "notifications.needsHuman",
  progressMilestone: "notifications.progressMilestone", cooked: "notifications.cooked",
  failed: "notifications.failed", disconnected: "notifications.disconnected",
  connectionRestored: "notifications.connectionRestored", stoveCleared: "notifications.stoveCleared",
};

const LOCAL_CHANNELS: LocalNotificationChannel[] = ["sound", "systemBanner", "barFlash", "systemAttention"];
const CHANNEL_KEYS: Record<LocalNotificationChannel, TranslationKey> = {
  sound: "notifications.sound", systemBanner: "notifications.system",
  barFlash: "notifications.flash", systemAttention: "notifications.attention",
};

type StatusMessage = {
  key: TranslationKey;
  name?: string;
  nameKey?: TranslationKey;
};

const DEFAULT_LOCAL_NOTIFICATION_SETTINGS: LocalNotificationSettingsWire = {
  sound: true,
  systemBanner: false,
  barFlash: false,
  systemAttention: false,
  events: ["needsHuman", "cooked", "failed", "disconnected"],
};

export function NotificationSettingsPanel() {
  const { t } = useI18n();
  const eventLabel = (event: NotificationEvent) => t(EVENT_KEYS[event]);
  const channelLabel = (channel: LocalNotificationChannel) => t(CHANNEL_KEYS[channel]);
  const [tab, setTab] = useState<"general" | "archive">("general");
  const [destinations, setDestinations] = useState<NotificationDestinationWire[]>([]);
  const [localSettings, setLocalSettings] = useState<LocalNotificationSettingsWire>(
    DEFAULT_LOCAL_NOTIFICATION_SETTINGS,
  );
  const [secrets, setSecrets] = useState<Partial<Record<NotificationDestination, string>>>({});
  const [busy, setBusy] = useState<NotificationDestination | null>(null);
  const [localBusy, setLocalBusy] = useState<"save" | LocalNotificationChannel | null>(null);
  const [status, setStatus] = useState<StatusMessage | null>(null);

  useEffect(() => {
    void getNotificationSettings().then(setDestinations).catch(() => {
      setStatus({ key: "notifications.settingsUnavailable" });
    });
    void getLocalNotificationSettings().then(setLocalSettings).catch(() => {
      setStatus({ key: "notifications.localUnavailable" });
    });
  }, []);

  useEffect(() => {
    if (!status) return undefined;
    const timeout = window.setTimeout(() => setStatus(null), 20_000);
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
    setStatus(null);
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
      setStatus({ key: "notifications.saved", name: LABELS[item.destination] });
    } catch {
      setStatus({ key: "notifications.saveFailed", name: LABELS[item.destination] });
    } finally {
      setBusy(null);
    }
  };

  const test = async (item: NotificationDestinationWire) => {
    setBusy(item.destination);
    setStatus(null);
    try {
      await sendTestNotification(item.destination);
      setStatus({ key: "notifications.testSent", name: LABELS[item.destination] });
    } catch {
      setStatus({ key: "notifications.testFailed", name: LABELS[item.destination] });
    } finally {
      setBusy(null);
    }
  };

  const updateLocal = (change: Partial<LocalNotificationSettingsWire>) => {
    setLocalSettings((current) => ({ ...current, ...change }));
  };

  const saveLocal = async () => {
    setLocalBusy("save");
    setStatus(null);
    try {
      setLocalSettings(await configureLocalNotificationSettings(localSettings));
      setStatus({ key: "notifications.localSaved" });
    } catch {
      setStatus({ key: "notifications.localSaveFailed" });
    } finally {
      setLocalBusy(null);
    }
  };

  const testLocal = async (channel: LocalNotificationChannel) => {
    setLocalBusy(channel);
    setStatus(null);
    try {
      const result = await testLocalNotification(channel);
      setStatus(
        result === "delivered" || result === "queued"
          ? { key: "notifications.testSent", nameKey: CHANNEL_KEYS[channel] }
          : result === "permissionDenied"
            ? { key: "notifications.permission", nameKey: CHANNEL_KEYS[channel] }
            : { key: "notifications.channelUnavailable", nameKey: CHANNEL_KEYS[channel] },
      );
    } catch {
      setStatus({ key: "notifications.localTestFailed" });
    } finally {
      setLocalBusy(null);
    }
  };

  return (
    <main className="notification-settings" aria-label={`Cookbench ${t("settings.title").toLowerCase()}`}>
      <div className="notification-settings__surface">
        <header className="notification-settings__masthead">
          <div>
            <p>Cookbench</p>
            <h1>{t("settings.title")}</h1>
          </div>
        </header>
        <div className="notification-settings__tabs" role="tablist" aria-label={t("settings.title")}>
          <button type="button" role="tab" aria-selected={tab === "general"} onClick={() => setTab("general")}>{t("settings.general")}</button>
          <button type="button" role="tab" aria-selected={tab === "archive"} onClick={() => setTab("archive")}>{t("settings.archive")}</button>
        </div>
        {tab === "archive" ? <ArchiveSettingsPanel /> : <>
        <DisplaySettingsPanel />
        <section aria-labelledby="local-alerts-title">
          <div className="notification-settings__section-heading">
            <h2 id="local-alerts-title">{t("notifications.local")}</h2>
          </div>
          <div className="notification-settings__local-alerts">
            {LOCAL_CHANNELS.map((channel) => (
              <div className="notification-settings__local-channel" key={channel}>
                <label className="notification-settings__toggle">
                  <input
                    type="checkbox"
                    checked={localSettings[channel]}
                    onChange={(event) => updateLocal({ [channel]: event.target.checked })}
                  />
                  <span>{channelLabel(channel)}</span>
                </label>
                <button
                  type="button"
                  aria-label={`${t("notifications.test")} ${channelLabel(channel)}`}
                  disabled={localBusy !== null}
                  onClick={() => void testLocal(channel)}
                >
                  {t("notifications.test")}
                </button>
              </div>
            ))}
            <fieldset>
              <legend>{t("common.states")}</legend>
              {EVENTS.map((event) => (
                <label key={event}>
                  <input
                    type="checkbox"
                    checked={localSettings.events.includes(event)}
                    onChange={(input) => updateLocal({
                      events: input.target.checked
                        ? [...localSettings.events, event]
                        : localSettings.events.filter((candidate) => candidate !== event),
                    })}
                  />
                  {eventLabel(event)}
                </label>
              ))}
            </fieldset>
            <div className="notification-settings__actions">
              <button type="button" disabled={localBusy !== null} onClick={() => void saveLocal()}>{t("common.save")}</button>
            </div>
          </div>
        </section>
        <section aria-labelledby="notification-settings-title">
          <div className="notification-settings__section-heading">
            <h2 id="notification-settings-title">{t("notifications.title")}</h2>
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
                <span>{t("common.enabled")}</span>
              </label>
            </div>
            <div className="notification-settings__fields">
              <label>
                <span>{item.destination === "telegram" ? t("notifications.botToken") : t("notifications.webhook")}</span>
                <input
                  type="password"
                  value={secrets[item.destination] ?? ""}
                  autoComplete="off"
                  placeholder={item.configured ? t("notifications.stored") : t("notifications.notConfigured")}
                  onChange={(event) => setSecrets((current) => ({
                    ...current,
                    [item.destination]: event.target.value,
                  }))}
                />
              </label>
              {item.destination === "telegram" ? (
                <label>
                  <span>{t("notifications.chatId")}</span>
                  <input
                    value={item.recipient ?? ""}
                    onChange={(event) => update(item.destination, { recipient: event.target.value })}
                  />
                </label>
              ) : null}
            </div>
            <fieldset>
              <legend>{t("common.states")}</legend>
              {EVENTS.map((event) => (
                <label key={event}>
                  <input
                    type="checkbox"
                    checked={item.events.includes(event)}
                    onChange={(input) => update(item.destination, {
                      events: input.target.checked
                        ? [...item.events, event]
                        : item.events.filter((candidate) => candidate !== event),
                    })}
                  />
                  {eventLabel(event)}
                </label>
              ))}
            </fieldset>
            <label className="notification-settings__template">
              <span>{t("notifications.template")}</span>
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
              <button type="button" disabled={busy === item.destination} onClick={() => void save(item)}>{t("common.save")}</button>
              <button
                type="button"
                disabled={!item.enabled || !item.configured || busy === item.destination}
                onClick={() => void test(item)}
              >
                {t("notifications.test")}
              </button>
            </div>
          </section>
        ))}
          </div>
          <output role="status" aria-live="polite">{
            status
              ? t(status.key, status.name || status.nameKey
                ? { name: status.name ?? (status.nameKey ? t(status.nameKey) : "") }
                : undefined)
              : ""
          }</output>
        </section>
        <SourcesStatusPanel />
        <HookHealthPanel />
        <RemoteSourcesPanel />
        </>}
      </div>
    </main>
  );
}
