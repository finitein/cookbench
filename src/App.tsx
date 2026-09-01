import { useEffect, useState } from "react";

import { DetachedStoveWindow } from "./components/DetachedStoveWindow";
import { GlobalBar } from "./components/GlobalBar";
import { LocatorActivationNotice } from "./components/LocatorActivationNotice";
import { useDetachedWindowStove } from "./hooks/useDetachedWindow";
import { useDisplaySettings } from "./hooks/useDisplaySettings";
import { useGlobalBarWindow } from "./hooks/useGlobalBarWindow";
import { useStoves } from "./hooks/useStoves";
import { detachedStoveTransport } from "./services/detachedStoves";
import { activateStove, type LocatorActivationResult } from "./services/locator";
import { acknowledgeCookedStove, archiveStove, clearCookedStove, setStovePinned } from "./services/stoves";
import { NotificationSettingsPanel } from "./settings/notifications/NotificationSettingsPanel";
import { openNotificationSettings } from "./settings/notifications/service";
import { useLocalAlert } from "./services/localAlerts";
import type { StoveWire } from "./types/stove";
import { I18nProvider, useI18n } from "./i18n/i18n";
import { syncNativeLocale } from "./settings/display/service";

export default function App() {
  const displaySettings = useDisplaySettings();
  return <I18nProvider preference={displaySettings?.locale}><CookbenchApp displaySettings={displaySettings} /></I18nProvider>;
}

function CookbenchApp({ displaySettings }: { displaySettings: ReturnType<typeof useDisplaySettings> }) {
  const { locale, t } = useI18n();
  const { stoves } = useStoves();
  const detached = useDetachedWindowStove(stoves);
  const { activeStoveId: activeAlertStoveId, dismiss: dismissLocalAlert } = useLocalAlert();
  useGlobalBarWindow();
  const [activation, setActivation] = useState<LocatorActivationResult | null>(null);
  useEffect(() => {
    if (!displaySettings) return;
    void syncNativeLocale(locale).catch(() => {
      // Browser fixtures do not expose the native Tauri command surface.
    });
  }, [displaySettings, locale]);
  const activate = async (stove: StoveWire) => {
    dismissLocalAlert(stove.id);
    if (stove.state === "cooked") {
      await acknowledgeCookedStove(stove.id).catch(() => undefined);
    }
    await activateStove(stove.id)
      .then(setActivation)
      .catch(() => setActivation({ target: "unavailable", status: "unavailable", resumeSessionId: null }));
  };

  if (detached.isSettings) {
    return <NotificationSettingsPanel />;
  }

  if (detached.isDetached) {
    return detached.stove
      ? <DetachedStoveWindow stove={detached.stove} onActivate={activate} activeAlertStoveId={activeAlertStoveId} />
      : <main className="shell shell--detached" aria-label={t("bar.detached", { name: "Cookbench" })} />;
  }

  return (
    <main className="shell shell--global-bar" aria-label="Cookbench">
      <GlobalBar
        stoves={stoves}
        onActivateStove={activate}
        onDetachStove={(stove) => { void detachedStoveTransport.detach(stove.id); }}
        onClearStove={(stove) => { void clearCookedStove(stove.id); }}
        onPinStove={(stove) => { void setStovePinned(stove.id, !stove.pinned); }}
        onArchiveStove={(stove) => { void archiveStove(stove.id); }}
        onOpenSettings={() => { void openNotificationSettings(); }}
        hoverDetailsEnabled={displaySettings?.hoverDetailsEnabled ?? false}
        activeAlertStoveId={activeAlertStoveId}
      />
      <LocatorActivationNotice result={activation} />
    </main>
  );
}
