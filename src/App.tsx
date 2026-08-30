import { useState } from "react";

import { DetachedStoveWindow } from "./components/DetachedStoveWindow";
import { GlobalBar } from "./components/GlobalBar";
import { LocatorActivationNotice } from "./components/LocatorActivationNotice";
import { useDetachedWindowStove } from "./hooks/useDetachedWindow";
import { useGlobalBarWindow } from "./hooks/useGlobalBarWindow";
import { useStoves } from "./hooks/useStoves";
import { detachedStoveTransport } from "./services/detachedStoves";
import { activateStove, type LocatorActivationResult } from "./services/locator";
import { clearCookedStove } from "./services/stoves";
import { NotificationSettingsPanel } from "./settings/notifications/NotificationSettingsPanel";
import { openNotificationSettings } from "./settings/notifications/service";
import type { StoveWire } from "./types/stove";

export default function App() {
  const { stoves } = useStoves();
  const detached = useDetachedWindowStove(stoves);
  useGlobalBarWindow();
  const [activation, setActivation] = useState<LocatorActivationResult | null>(null);
  const activate = (stove: StoveWire) => {
    void activateStove(stove.id)
      .then(setActivation)
      .catch(() => setActivation({ target: "unavailable", status: "unavailable", resumeSessionId: null }));
  };

  if (detached.isSettings) {
    return <NotificationSettingsPanel />;
  }

  if (detached.isDetached) {
    return detached.stove
      ? <DetachedStoveWindow stove={detached.stove} onActivate={activate} />
      : <main className="shell shell--detached" aria-label="Cookbench detached Stove" />;
  }

  return (
    <main className="shell shell--global-bar" aria-label="Cookbench">
      <GlobalBar
        stoves={stoves}
        onActivateStove={activate}
        onDetachStove={(stove) => { void detachedStoveTransport.detach(stove.id); }}
        onClearStove={(stove) => { void clearCookedStove(stove.id); }}
        onOpenSettings={() => { void openNotificationSettings(); }}
      />
      <LocatorActivationNotice result={activation} />
    </main>
  );
}
