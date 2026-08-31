import { useEffect, useState } from "react";

import { getHookStatus, manageHook, type HookAction, type HookStatus } from "./service";
import "./hook-health.css";

const HEALTH_LABEL: Record<HookStatus["health"], string> = {
  detected: "Native monitoring",
  notInstalled: "Not installed",
  healthy: "Healthy",
  outdated: "Outdated",
  conflicted: "Configuration conflict",
  unwritable: "Cannot write",
  noRecentEvents: "No recent events",
};

const TIER_LABEL: Record<HookStatus["tier"], string> = {
  full: "Full",
  standard: "Standard",
  experimental: "Experimental",
};

const INTEGRATION_LABEL: Record<HookStatus["integration"], string> = {
  automatic: "Automatic",
  manual: "Manual setup",
  presenceOnly: "Presence only",
};

export function HookHealthPanel() {
  const [hooks, setHooks] = useState<HookStatus[]>([]);
  const [message, setMessage] = useState("");
  const [preview, setPreview] = useState<string | null>(null);

  const refresh = () => {
    void getHookStatus().then(setHooks).catch(() => setMessage("Hook status is unavailable."));
  };

  useEffect(refresh, []);

  const act = (harness: HookStatus["harness"], action: HookAction) => {
    setMessage("");
    void manageHook(harness, action).then((result) => {
      if (result.preview !== null) {
        setPreview(result.preview);
        setMessage("Preview only. No harness configuration was changed.");
      } else {
        setPreview(null);
        setMessage(result.changed ? "Hook configuration updated." : "Hook configuration already matches this action.");
      }
      refresh();
    }).catch(() => setMessage("Cookbench could not update this hook safely."));
  };

  return (
    <section className="hook-health" aria-labelledby="hook-health-title">
      <header>
        <h2 id="hook-health-title">Hook Health</h2>
        <p>Optional lifecycle signals. Native session files remain authoritative.</p>
      </header>
      <ul className="hook-health__list" aria-label="Hook health">
        {hooks.map((hook) => (
          <li key={hook.harness} className="hook-health__item">
            <div className="hook-health__identity">
              <div className="hook-health__title">
                <strong>{hook.label}</strong>
                <span className={`hook-health__tier hook-health__tier--${hook.tier}`}>{TIER_LABEL[hook.tier]}</span>
                <span className="hook-health__integration">{INTEGRATION_LABEL[hook.integration]}</span>
              </div>
              <span title={hook.configDisplay}>{hook.configDisplay}</span>
            </div>
            <div className="hook-health__detail">
              <span className={`hook-health__state hook-health__state--${hook.health}`}>{HEALTH_LABEL[hook.health]}</span>
              <span>{hook.detail}</span>
            </div>
            <div className="hook-health__actions" aria-label={`${hook.label} hook actions`}>
              {hook.canInstall ? <button type="button" onClick={() => act(hook.harness, "previewInstall")}>Preview</button> : null}
              {hook.canInstall ? <button type="button" onClick={() => act(hook.harness, "install")}>Install</button> : null}
              {hook.canRepair ? <button type="button" onClick={() => act(hook.harness, "repair")}>Repair</button> : null}
              {hook.canUninstall ? <button type="button" onClick={() => act(hook.harness, "uninstall")}>Uninstall</button> : null}
            </div>
          </li>
        ))}
      </ul>
      {preview !== null ? <pre className="hook-health__preview" aria-label="Hook configuration preview">{preview}</pre> : null}
      <output role="status" aria-live="polite">{message}</output>
    </section>
  );
}
