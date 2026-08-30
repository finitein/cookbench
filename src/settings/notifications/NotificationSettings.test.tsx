import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { NotificationSettings } from "./NotificationSettings";
import { NotificationSettingsPanel } from "./NotificationSettingsPanel";

vi.mock("../hooks/HookHealthPanel", () => ({
  HookHealthPanel: () => <section aria-label="Hook health integration" />,
}));

vi.mock("./service", () => ({
  configureNotificationDestination: vi.fn(),
  configureLocalNotificationSettings: vi.fn(async (input) => input),
  getLocalNotificationSettings: vi.fn(async () => ({
    sound: true,
    systemBanner: false,
    barFlash: false,
    systemAttention: false,
    events: ["needsHuman", "cooked", "failed", "disconnected"],
  })),
  getNotificationSettings: vi.fn(async () => []),
  sendTestNotification: vi.fn(),
  testLocalNotification: vi.fn(async () => "delivered"),
}));

describe("NotificationSettings", () => {
  it("uses a dedicated solid settings surface instead of inheriting the floating Bar layout", async () => {
    render(<NotificationSettingsPanel />);

    await waitFor(() => expect(screen.getByRole("main", { name: "Cookbench settings" })).toBeInTheDocument());
    expect(screen.getByText("Settings", { selector: "h1" })).toBeInTheDocument();
    expect(document.querySelector(".notification-settings__surface")).toBeInTheDocument();
    expect(screen.getByLabelText("Hook health integration")).toBeInTheDocument();
  });

  it("only exposes outbound destination toggles and synthetic test sends", async () => {
    const onChange = vi.fn();
    const onTest = vi.fn(async () => {});
    render(<NotificationSettings destinations={[{ destination: "telegram", enabled: false, secretReference: "secret://Cookbench/telegram" }]} onChange={onChange} onTest={onTest} />);
    expect(screen.getByRole("button", { name: "Send Telegram test notification" })).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: "Telegram" }));
    expect(onChange).toHaveBeenCalledWith([{ destination: "telegram", enabled: true, secretReference: "secret://Cookbench/telegram" }]);
    expect(screen.queryByText("secret://Cookbench/telegram")).not.toBeInTheDocument();
  });

  it("defaults local alerts to sound and saves shared event choices", async () => {
    const service = await import("./service");
    render(<NotificationSettingsPanel />);

    const sound = await screen.findByRole("checkbox", { name: "Sound" });
    expect(sound).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "System notification" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Flash Stove" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Request attention" })).not.toBeChecked();

    fireEvent.click(screen.getByRole("checkbox", { name: "Cooking Started" }));
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(service.configureLocalNotificationSettings).toHaveBeenCalledWith(expect.objectContaining({
      sound: true,
      events: expect.arrayContaining(["cookingStarted", "needsHuman", "cooked", "failed", "disconnected"]),
    })));
    expect(screen.getByText("Local alerts saved.")).toBeInTheDocument();
  });

  it("tests each local alert channel and explains a denied notification permission", async () => {
    const service = await import("./service");
    vi.mocked(service.testLocalNotification).mockResolvedValueOnce("permissionDenied");
    render(<NotificationSettingsPanel />);

    const section = screen.getByRole("heading", { name: "Local alerts" }).closest("section");
    expect(section).not.toBeNull();
    fireEvent.click(within(section!).getByRole("button", { name: "Test System notification" }));

    await waitFor(() => expect(service.testLocalNotification).toHaveBeenCalledWith("systemBanner"));
    expect(screen.getByText("System notification needs system notification permission.")).toBeInTheDocument();
  });

  it("removes local alert feedback after twenty seconds", async () => {
    render(<NotificationSettingsPanel />);
    const section = (await screen.findByRole("heading", { name: "Local alerts" })).closest("section");
    expect(section).not.toBeNull();

    vi.useFakeTimers();
    try {
      await act(async () => {
        fireEvent.click(within(section!).getByRole("button", { name: "Test Sound" }));
      });
      expect(screen.getByText("Sound test sent.")).toBeInTheDocument();

      act(() => vi.advanceTimersByTime(20_000));
      expect(screen.queryByText("Sound test sent.")).not.toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps archive recovery in its own Settings tab", async () => {
    render(<NotificationSettingsPanel />);

    const archive = screen.getByRole("tab", { name: "Archive" });
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true");
    fireEvent.click(archive);
    await waitFor(() => expect(archive).toHaveAttribute("aria-selected", "true"));
    expect(screen.getByRole("heading", { name: "Archive" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Notifications" })).not.toBeInTheDocument();
  });

  it("runs a test only for an enabled channel", async () => {
    const onTest = vi.fn(async () => {});
    render(<NotificationSettings destinations={[{ destination: "slack", enabled: true, secretReference: null }]} onChange={() => {}} onTest={onTest} />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Send Slack test notification" }));
    });
    expect(onTest).toHaveBeenCalledWith("slack");
  });
});
