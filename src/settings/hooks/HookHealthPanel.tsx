import { useEffect, useState } from "react";

import { getHookStatus, manageHook, type HookAction, type HookHealth, type HookIntegration, type HookStatus, type HookSupportTier } from "./service";
import { useI18n, type TranslationKey } from "../../i18n/i18n";
import "./hook-health.css";

const TIER_KEYS: Record<HookSupportTier, TranslationKey> = {
  full: "sources.full", standard: "sources.standard", experimental: "sources.experimental",
};
const INTEGRATION_KEYS: Record<HookIntegration, TranslationKey> = {
  automatic: "hooks.automatic", manual: "hooks.manual", presenceOnly: "hooks.presenceOnly",
};
const HEALTH_KEYS: Record<HookHealth, TranslationKey> = {
  detected: "hooks.detected", notInstalled: "hooks.notInstalled", healthy: "hooks.healthy",
  outdated: "hooks.outdated", conflicted: "hooks.conflicted", unwritable: "hooks.unwritable",
  noRecentEvents: "hooks.noRecentEvents",
};

export function HookHealthPanel() {
  const { t } = useI18n();
  const [hooks, setHooks] = useState<HookStatus[]>([]);
  const [message, setMessage] = useState<TranslationKey | null>(null);
  const [preview, setPreview] = useState<string | null>(null);

  const refresh = () => {
    void getHookStatus().then(setHooks).catch(() => setMessage("hooks.unavailable"));
  };

  useEffect(refresh, []);

  const act = (harness: HookStatus["harness"], action: HookAction) => {
    setMessage(null);
    void manageHook(harness, action).then((result) => {
      if (result.preview !== null) {
        setPreview(result.preview);
        setMessage("hooks.preview");
      } else {
        setPreview(null);
        setMessage(result.changed ? "hooks.updated" : "hooks.matches");
      }
      refresh();
    }).catch(() => setMessage("hooks.failed"));
  };

  return (
    <section className="hook-health" aria-labelledby="hook-health-title">
      <header>
        <h2 id="hook-health-title">{t("hooks.title")}</h2>
        <p>{t("hooks.description")}</p>
      </header>
      <ul className="hook-health__list" aria-label={t("hooks.list")}>
        {hooks.map((hook) => (
          <li key={hook.harness} className="hook-health__item">
            <div className="hook-health__identity">
              <div className="hook-health__title">
                <strong>{hook.label}</strong>
                <span className={`hook-health__tier hook-health__tier--${hook.tier}`}>{t(TIER_KEYS[hook.tier])}</span>
                <span className="hook-health__integration">{t(INTEGRATION_KEYS[hook.integration])}</span>
              </div>
              <span title={hook.configDisplay}>{hook.configDisplay}</span>
            </div>
            <div className="hook-health__detail">
              <span className={`hook-health__state hook-health__state--${hook.health}`}>{t(HEALTH_KEYS[hook.health])}</span>
              <span>{hook.detail}</span>
            </div>
            <div className="hook-health__actions" aria-label={t("hooks.actions", { name: hook.label })}>
              {hook.canInstall ? <button type="button" onClick={() => act(hook.harness, "previewInstall")}>{t("common.preview")}</button> : null}
              {hook.canInstall ? <button type="button" onClick={() => act(hook.harness, "install")}>{t("common.install")}</button> : null}
              {hook.canRepair ? <button type="button" onClick={() => act(hook.harness, "repair")}>{t("common.repair")}</button> : null}
              {hook.canUninstall ? <button type="button" onClick={() => act(hook.harness, "uninstall")}>{t("common.uninstall")}</button> : null}
            </div>
          </li>
        ))}
      </ul>
      {preview !== null ? <pre className="hook-health__preview" aria-label={t("hooks.previewLabel")}>{preview}</pre> : null}
      <output role="status" aria-live="polite">{message ? t(message) : ""}</output>
    </section>
  );
}
