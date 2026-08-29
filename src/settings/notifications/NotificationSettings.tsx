import { useState } from "react";

export type NotificationDestination = "telegram" | "slack" | "discord" | "lark" | "generic";

export interface NotificationDestinationSettings {
  destination: NotificationDestination;
  enabled: boolean;
  secretReference: string | null;
}

export interface NotificationSettingsProps {
  destinations: NotificationDestinationSettings[];
  onChange: (destinations: NotificationDestinationSettings[]) => void;
  onTest: (destination: NotificationDestination) => Promise<void>;
}

const LABELS: Record<NotificationDestination, string> = {
  telegram: "Telegram",
  slack: "Slack",
  discord: "Discord",
  lark: "Lark / Feishu",
  generic: "Generic Webhook",
};

export function NotificationSettings({ destinations, onChange, onTest }: NotificationSettingsProps) {
  const [testing, setTesting] = useState<NotificationDestination | null>(null);
  const update = (destination: NotificationDestination, enabled: boolean) => {
    onChange(destinations.map((item) => item.destination === destination ? { ...item, enabled } : item));
  };
  const test = async (destination: NotificationDestination) => {
    setTesting(destination);
    try { await onTest(destination); } finally { setTesting(null); }
  };

  return (
    <section aria-label="Outbound notifications">
      {destinations.map((item) => (
        <div key={item.destination}>
          <label>
            <input
              type="checkbox"
              checked={item.enabled}
              onChange={(event) => update(item.destination, event.target.checked)}
            />
            {LABELS[item.destination]}
          </label>
          <button
            type="button"
            aria-label={`Send ${LABELS[item.destination]} test notification`}
            disabled={!item.enabled || testing === item.destination}
            onClick={() => void test(item.destination)}
          >
            Test
          </button>
        </div>
      ))}
    </section>
  );
}
