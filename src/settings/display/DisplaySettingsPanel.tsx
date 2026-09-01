import { useEffect, useRef, useState } from "react";

import {
  closeDetachedBar,
  configureDisplaySettings,
  getDisplaySettings,
  getLaunchAtLogin,
  setLaunchAtLogin,
  type DisplaySettingsWire,
  type GlobalBarMode,
  type GlobalBarPlacement,
  type AppLocale,
} from "./service";
import { useI18n, type TranslationKey } from "../../i18n/i18n";
import "./display-settings.css";

const PLACEMENTS: GlobalBarPlacement[] = ["topLeft", "topCenter", "topRight", "bottomLeft", "bottomCenter", "bottomRight"];
const LOCALES: AppLocale[] = ["system", "en", "zh-CN", "ja", "ko"];
const PLACEMENT_KEYS: Record<GlobalBarPlacement, TranslationKey> = {
  topLeft: "display.topLeft", topCenter: "display.topCenter", topRight: "display.topRight",
  bottomLeft: "display.bottomLeft", bottomCenter: "display.bottomCenter", bottomRight: "display.bottomRight",
};
const LOCALE_KEYS: Record<AppLocale, TranslationKey> = {
  system: "language.system", en: "language.en", "zh-CN": "language.zh-CN", ja: "language.ja", ko: "language.ko",
};
const MODE_KEYS: Record<GlobalBarMode, TranslationKey> = {
  full: "display.modeFull",
  minimal: "display.modeMinimal",
};

export function DisplaySettingsPanel() {
  const { t } = useI18n();
  const [settings, setSettings] = useState<DisplaySettingsWire | null>(null);
  const [status, setStatus] = useState<TranslationKey | null>(null);
  const [closing, setClosing] = useState<string | null>(null);
  const [launchAtLogin, setLaunchAtLoginState] = useState<boolean | null>(null);
  const desiredSettings = useRef<DisplaySettingsWire | null>(null);
  const saveGeneration = useRef(0);
  const saveQueue = useRef(Promise.resolve());

  useEffect(() => {
    void getDisplaySettings().then((loaded) => {
      desiredSettings.current = loaded;
      setSettings(loaded);
    }).catch(() => {
      setStatus("display.unavailable");
    });
    void getLaunchAtLogin().then((preference) => {
      setLaunchAtLoginState(preference.enabled);
    }).catch(() => {
      setStatus("display.loginUnavailable");
    });
  }, []);

  const save = (change: Partial<Pick<
    DisplaySettingsWire,
    "globalBarVisible" | "globalBarPlacement" | "globalBarMode" | "macStatusStoveCount" | "hoverDetailsEnabled" | "locale"
  >>) => {
    const current = desiredSettings.current ?? settings;
    if (!current) return;
    const next = { ...current, ...change };
    desiredSettings.current = next;
    setSettings(next);
    setStatus(null);
    const generation = ++saveGeneration.current;
    const input = {
      globalBarVisible: next.globalBarVisible,
      globalBarPlacement: next.globalBarPlacement,
      globalBarMode: next.globalBarMode,
      macStatusStoveCount: next.macStatusStoveCount,
      hoverDetailsEnabled: next.hoverDetailsEnabled,
      locale: next.locale,
    };
    saveQueue.current = saveQueue.current.then(async () => {
      try {
        const saved = await configureDisplaySettings(input);
        if (generation === saveGeneration.current) {
          desiredSettings.current = saved;
          setSettings(saved);
        }
      } catch {
        if (generation !== saveGeneration.current) return;
        setStatus("display.saveFailed");
        try {
          const authoritative = await getDisplaySettings();
          if (generation === saveGeneration.current) {
            desiredSettings.current = authoritative;
            setSettings(authoritative);
          }
        } catch {
          if (generation === saveGeneration.current) setStatus("display.saveFailed");
        }
      }
    });
  };

  const close = (stoveId: string) => {
    setClosing(stoveId);
    setStatus(null);
    void closeDetachedBar(stoveId).then((closed) => {
      if (closed) {
        setSettings((current) => current && {
          ...current,
          detachedBars: current.detachedBars.filter((bar) => bar.stoveId !== stoveId),
        });
      }
    }).catch(() => {
      setStatus("display.closeFailed");
    }).finally(() => setClosing(null));
  };

  return (
    <section className="display-settings" aria-labelledby="display-settings-title">
      <header>
        <h2 id="display-settings-title">{t("display.title")}</h2>
      </header>
      <section className="display-settings__section" aria-labelledby="global-bar-title">
        <div>
          <h3 id="global-bar-title">{t("display.global")}</h3>
          <p>{t("display.globalDescription")}</p>
        </div>
        <label className="display-settings__toggle">
          <input
            type="checkbox"
            checked={settings?.globalBarVisible ?? false}
            disabled={!settings}
            onChange={(event) => save({ globalBarVisible: event.target.checked })}
          />
          <span>{t("display.showGlobal")}</span>
        </label>
        <label className="display-settings__toggle">
          <input
            type="checkbox"
            checked={settings?.hoverDetailsEnabled ?? false}
            disabled={!settings || !settings.globalBarVisible}
            onChange={(event) => save({ hoverDetailsEnabled: event.target.checked })}
          />
          <span>{t("display.hover")}</span>
        </label>
        <label className="display-settings__placement">
          <span>{t("display.placement")}</span>
          <select
            value={settings?.globalBarPlacement ?? "topCenter"}
            disabled={!settings || !settings.globalBarVisible}
            onChange={(event) => save({ globalBarPlacement: event.target.value as GlobalBarPlacement })}
          >
            {PLACEMENTS.map((placement) => <option key={placement} value={placement}>{t(PLACEMENT_KEYS[placement])}</option>)}
          </select>
        </label>
        <fieldset className="display-settings__mode" disabled={!settings || !settings.globalBarVisible}>
          <legend>{t("display.mode")}</legend>
          <div role="radiogroup" aria-label={t("display.mode")}>
            {(Object.keys(MODE_KEYS) as GlobalBarMode[]).map((mode) => (
              <label key={mode} className="display-settings__mode-option">
                <input
                  type="radio"
                  name="global-bar-mode"
                  checked={(settings?.globalBarMode ?? "full") === mode}
                  onChange={() => save({ globalBarMode: mode })}
                />
                <span aria-hidden="true">{mode === "full" ? "[]" : "o"}</span>
                <span>{t(MODE_KEYS[mode])}</span>
              </label>
            ))}
          </div>
        </fieldset>
      </section>
      {settings?.macStatusAvailable ? (
        <section className="display-settings__section" aria-labelledby="mac-status-title">
          <div>
            <h3 id="mac-status-title">{t("display.macStatus")}</h3>
            <p>{t("display.macStatusDescription")}</p>
          </div>
          <label className="display-settings__placement" htmlFor="mac-status-stove-count">
            <span>{t("display.macStatusCount")}</span>
            <input
              id="mac-status-stove-count"
              aria-label={t("display.macStatusCount")}
              type="number"
              min={0}
              max={8}
              step={1}
              value={settings.macStatusStoveCount}
              aria-describedby="mac-status-count-help"
              onChange={(event) => {
                const count = Number(event.target.value);
                if (Number.isInteger(count) && count >= 0 && count <= 8) {
                  save({ macStatusStoveCount: count });
                }
              }}
            />
            <small id="mac-status-count-help">{t("display.macStatusOff")}</small>
          </label>
        </section>
      ) : null}
      <section className="display-settings__section" aria-labelledby="language-title">
        <div>
          <h3 id="language-title">{t("language.title")}</h3>
          <p>{t("language.description")}</p>
        </div>
        <label className="display-settings__placement">
          <span>{t("language.title")}</span>
          <select
            value={settings?.locale ?? "system"}
            disabled={!settings}
            onChange={(event) => save({ locale: event.target.value as AppLocale })}
          >
            {LOCALES.map((locale) => <option key={locale} value={locale}>{t(LOCALE_KEYS[locale])}</option>)}
          </select>
        </label>
      </section>
      <section className="display-settings__section" aria-labelledby="desktop-behavior-title">
        <div>
          <h3 id="desktop-behavior-title">{t("display.desktop")}</h3>
          <p>{t("display.desktopDescription")}</p>
        </div>
        <label className="display-settings__toggle">
          <input
            type="checkbox"
            checked={launchAtLogin ?? false}
            disabled={launchAtLogin === null}
            onChange={(event) => {
              const previous = launchAtLogin;
              const enabled = event.target.checked;
              setLaunchAtLoginState(enabled);
              setStatus(null);
              void setLaunchAtLogin(enabled)
                .then((preference) => setLaunchAtLoginState(preference.enabled))
                .catch(() => {
                  setLaunchAtLoginState(previous);
                  setStatus("display.loginFailed");
                });
            }}
          />
          <span>{t("display.login")}</span>
        </label>
      </section>
      <section className="display-settings__section" aria-labelledby="independent-bars-title">
        <div>
          <h3 id="independent-bars-title">{t("display.independent")}</h3>
          <p>{t("display.independentDescription")}</p>
        </div>
        {settings?.detachedBars.length ? (
          <ul className="display-settings__bars">
            {settings.detachedBars.map((bar) => (
              <li key={bar.stoveId}>
                <span>{t("display.independent")} {bar.stoveId}</span>
                <button
                  type="button"
                  disabled={closing === bar.stoveId}
                  aria-label={t("display.closeIndependent", { id: bar.stoveId })}
                  onClick={() => close(bar.stoveId)}
                >
                  {t("common.close")}
                </button>
              </li>
            ))}
          </ul>
        ) : <p className="display-settings__empty">{t("display.empty")}</p>}
      </section>
      <output role="status" aria-live="polite">{status ? t(status) : ""}</output>
    </section>
  );
}
