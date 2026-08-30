import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import {
  configureLocalNotificationSettings,
  getLocalNotificationSettings,
  sendTestNotification,
  testLocalNotification,
  type LocalNotificationSettingsWire,
} from "./service";

beforeEach(() => {
  invoke.mockReset();
});

describe("sendTestNotification", () => {
  it("uses only an opaque destination id and never a secret", async () => {
    invoke.mockResolvedValueOnce(undefined);
    await sendTestNotification("slack");
    expect(invoke).toHaveBeenCalledWith("send_test_notification", { destination: "slack" });
    expect(JSON.stringify(invoke.mock.calls)).not.toMatch(/token|webhook|https?:/i);
  });
});

describe("local notification service", () => {
  const settings: LocalNotificationSettingsWire = {
    sound: true,
    systemBanner: false,
    barFlash: false,
    systemAttention: false,
    events: ["needsHuman", "cooked", "failed", "disconnected"],
  };

  it("loads and saves only local preference fields", async () => {
    invoke.mockResolvedValueOnce(settings).mockResolvedValueOnce(settings);

    await expect(getLocalNotificationSettings()).resolves.toEqual(settings);
    await expect(configureLocalNotificationSettings(settings)).resolves.toEqual(settings);

    expect(invoke).toHaveBeenNthCalledWith(1, "get_local_notification_settings");
    expect(invoke).toHaveBeenNthCalledWith(2, "configure_local_notification_settings", { input: settings });
  });

  it("tests a local channel through an opaque channel id", async () => {
    invoke.mockResolvedValueOnce("queued");

    await expect(testLocalNotification("sound")).resolves.toBe("queued");
    expect(invoke).toHaveBeenCalledWith("test_local_notification", { channel: "sound" });
  });
});
