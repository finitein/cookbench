import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { NotificationSettings } from "./NotificationSettings";
import { NotificationSettingsPanel } from "./NotificationSettingsPanel";

vi.mock("../hooks/HookHealthPanel", () => ({
  HookHealthPanel: () => <section aria-label="Hook health integration" />,
}));

vi.mock("./service", () => ({
  configureNotificationDestination: vi.fn(),
  getNotificationSettings: vi.fn(async () => []),
  sendTestNotification: vi.fn(),
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

  it("runs a test only for an enabled channel", async () => {
    const onTest = vi.fn(async () => {});
    render(<NotificationSettings destinations={[{ destination: "slack", enabled: true, secretReference: null }]} onChange={() => {}} onTest={onTest} />);
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Send Slack test notification" }));
    });
    expect(onTest).toHaveBeenCalledWith("slack");
  });
});
