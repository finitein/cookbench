import { useEffect, useState } from "react";

import {
  closeDetachedBar,
  configureDisplaySettings,
  getDisplaySettings,
  type DisplaySettingsWire,
  type GlobalBarPlacement,
} from "./service";
import "./display-settings.css";

const PLACEMENTS: Array<{ value: GlobalBarPlacement; label: string }> = [
  { value: "topLeft", label: "Top left" },
  { value: "topCenter", label: "Top center" },
  { value: "topRight", label: "Top right" },
  { value: "bottomLeft", label: "Bottom left" },
  { value: "bottomCenter", label: "Bottom center" },
  { value: "bottomRight", label: "Bottom right" },
];

export function DisplaySettingsPanel() {
  const [settings, setSettings] = useState<DisplaySettingsWire | null>(null);
  const [status, setStatus] = useState("");
  const [closing, setClosing] = useState<string | null>(null);

  useEffect(() => {
    void getDisplaySettings().then(setSettings).catch(() => {
      setStatus("Display settings are unavailable.");
    });
  }, []);

  const save = (change: Partial<Pick<DisplaySettingsWire, "globalBarVisible" | "globalBarPlacement">>) => {
    if (!settings) return;
    const next = { ...settings, ...change };
    setSettings(next);
    setStatus("");
    void configureDisplaySettings({
      globalBarVisible: next.globalBarVisible,
      globalBarPlacement: next.globalBarPlacement,
    }).then(setSettings).catch(() => {
      setSettings(settings);
      setStatus("Display settings could not be saved.");
    });
  };

  const close = (stoveId: string) => {
    setClosing(stoveId);
    setStatus("");
    void closeDetachedBar(stoveId).then((closed) => {
      if (closed) {
        setSettings((current) => current && {
          ...current,
          detachedBars: current.detachedBars.filter((bar) => bar.stoveId !== stoveId),
        });
      }
    }).catch(() => {
      setStatus("Independent Bar could not be closed.");
    }).finally(() => setClosing(null));
  };

  return (
    <section className="display-settings" aria-labelledby="display-settings-title">
      <header>
        <h2 id="display-settings-title">Display</h2>
      </header>
      <section className="display-settings__section" aria-labelledby="global-bar-title">
        <div>
          <h3 id="global-bar-title">Global Bar</h3>
          <p>Shows every Stove in one place.</p>
        </div>
        <label className="display-settings__toggle">
          <input
            type="checkbox"
            checked={settings?.globalBarVisible ?? false}
            disabled={!settings}
            onChange={(event) => save({ globalBarVisible: event.target.checked })}
          />
          <span>Show global Bar</span>
        </label>
        <label className="display-settings__placement">
          <span>Placement</span>
          <select
            value={settings?.globalBarPlacement ?? "topCenter"}
            disabled={!settings || !settings.globalBarVisible}
            onChange={(event) => save({ globalBarPlacement: event.target.value as GlobalBarPlacement })}
          >
            {PLACEMENTS.map((placement) => <option key={placement.value} value={placement.value}>{placement.label}</option>)}
          </select>
        </label>
      </section>
      <section className="display-settings__section" aria-labelledby="independent-bars-title">
        <div>
          <h3 id="independent-bars-title">Independent Bars</h3>
          <p>Stay visible even when the global Bar is hidden.</p>
        </div>
        {settings?.detachedBars.length ? (
          <ul className="display-settings__bars">
            {settings.detachedBars.map((bar) => (
              <li key={bar.stoveId}>
                <span>Independent Bar {bar.stoveId}</span>
                <button
                  type="button"
                  disabled={closing === bar.stoveId}
                  aria-label={`Close independent Bar ${bar.stoveId}`}
                  onClick={() => close(bar.stoveId)}
                >
                  Close
                </button>
              </li>
            ))}
          </ul>
        ) : <p className="display-settings__empty">No independent Bars are open.</p>}
      </section>
      <output role="status" aria-live="polite">{status}</output>
    </section>
  );
}
