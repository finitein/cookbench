import { invoke } from "@tauri-apps/api/core";

import type { NotificationDestination } from "./NotificationSettings";

export type NotificationEvent =
  | "sessionAppeared"
  | "cookingStarted"
  | "phaseChanged"
  | "needsHuman"
  | "progressMilestone"
  | "cooked"
  | "failed"
  | "disconnected"
  | "connectionRestored"
  | "stoveCleared";

export type NotificationDestinationWire = {
  destination: NotificationDestination;
  enabled: boolean;
  configured: boolean;
  recipient: string | null;
  events: NotificationEvent[];
  template: string | null;
};

export type NotificationDestinationInput = {
  destination: NotificationDestination;
  enabled: boolean;
  secret: string | null;
  recipient: string | null;
  events: NotificationEvent[];
  template: string | null;
};

export type LocalNotificationChannel = "sound" | "systemBanner" | "barFlash" | "systemAttention";

export type LocalNotificationSettingsWire = {
  sound: boolean;
  systemBanner: boolean;
  barFlash: boolean;
  systemAttention: boolean;
  events: NotificationEvent[];
};

export type LocalNotificationTestResult = "delivered" | "permissionDenied" | "unavailable";

export function openNotificationSettings(): Promise<void> {
  return invoke<void>("open_notification_settings");
}

export function getNotificationSettings(): Promise<NotificationDestinationWire[]> {
  return invoke<NotificationDestinationWire[]>("get_notification_settings");
}

export function getLocalNotificationSettings(): Promise<LocalNotificationSettingsWire> {
  return invoke<LocalNotificationSettingsWire>("get_local_notification_settings");
}

export function configureLocalNotificationSettings(
  input: LocalNotificationSettingsWire,
): Promise<LocalNotificationSettingsWire> {
  return invoke<LocalNotificationSettingsWire>("configure_local_notification_settings", { input });
}

export function testLocalNotification(
  channel: LocalNotificationChannel,
): Promise<LocalNotificationTestResult> {
  return invoke<LocalNotificationTestResult>("test_local_notification", { channel });
}

export function configureNotificationDestination(
  input: NotificationDestinationInput,
): Promise<NotificationDestinationWire[]> {
  return invoke<NotificationDestinationWire[]>("configure_notification_destination", { input });
}

/** Calls the desktop's one-way synthetic test command; it never handles replies. */
export function sendTestNotification(destination: NotificationDestination): Promise<void> {
  return invoke<void>("send_test_notification", { destination });
}
