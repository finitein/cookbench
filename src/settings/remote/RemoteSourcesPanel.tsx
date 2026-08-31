import { useEffect, useState } from "react";

import {
  configureRemoteSource,
  getRemoteSources,
  removeRemoteSource,
  type RemoteSourceWire,
} from "./service";
import { useI18n, type TranslationKey } from "../../i18n/i18n";

export function RemoteSourcesPanel() {
  const { t } = useI18n();
  const [sources, setSources] = useState<RemoteSourceWire[]>([]);
  const [alias, setAlias] = useState("");
  const [roots, setRoots] = useState("");
  const [bridgeEnabled, setBridgeEnabled] = useState(false);
  const [bridgeBinaryPath, setBridgeBinaryPath] = useState("");
  const [status, setStatus] = useState<TranslationKey | null>(null);

  useEffect(() => {
    void getRemoteSources().then(setSources).catch(() => setStatus("remote.unavailable"));
  }, []);

  const add = async () => {
    setStatus(null);
    const sessionRoots = roots.split(",").map((root) => root.trim()).filter(Boolean);
    try {
      const next = await configureRemoteSource({
        id: null,
        alias: alias.trim(),
        sessionRoots,
        enabled: true,
        bridgeEnabled,
        bridgeBinaryPath: bridgeBinaryPath.trim() || null,
      });
      setSources(next);
      setAlias("");
      setStatus("remote.saved");
    } catch {
      setStatus("remote.saveFailed");
    }
  };

  const toggle = async (source: RemoteSourceWire) => {
    try {
      setSources(await configureRemoteSource({
        id: source.id,
        alias: source.alias,
        sessionRoots: source.sessionRoots,
        enabled: !source.enabled,
        bridgeEnabled: source.bridgeEnabled,
        bridgeBinaryPath: source.bridgeBinaryPath,
      }));
    } catch {
      setStatus("remote.updateFailed");
    }
  };

  const remove = async (source: RemoteSourceWire) => {
    try {
      setSources(await removeRemoteSource(source.id));
      setStatus("remote.removed");
    } catch {
      setStatus("remote.removeFailed");
    }
  };

  return (
    <section className="remote-settings" aria-labelledby="remote-settings-title">
      <h2 id="remote-settings-title">{t("remote.title")}</h2>
      <div className="remote-settings__add">
        <label>
          <span>{t("remote.alias")}</span>
          <input value={alias} onChange={(event) => setAlias(event.target.value)} autoComplete="off" />
        </label>
        <label className="remote-settings__bridge-choice">
          <input
            type="checkbox"
            checked={bridgeEnabled}
            onChange={(event) => setBridgeEnabled(event.target.checked)}
          />
          <span>{t("remote.bridge")}</span>
        </label>
        {bridgeEnabled ? (
          <label>
            <span>{t("remote.binary")}</span>
            <input
              value={bridgeBinaryPath}
              onChange={(event) => setBridgeBinaryPath(event.target.value)}
              autoComplete="off"
              placeholder={t("remote.packaged")}
            />
          </label>
        ) : null}
        <label>
          <span>{t("remote.roots")}</span>
          <input
            value={roots}
            onChange={(event) => setRoots(event.target.value)}
            autoComplete="off"
            placeholder={t("remote.automatic")}
          />
        </label>
        <button type="button" disabled={!alias.trim()} onClick={() => void add()}>{t("common.add")}</button>
      </div>
      <div className="remote-settings__sources">
        {sources.map((source) => (
          <div key={source.id} className="remote-settings__source">
            <div>
              <strong>{source.alias}</strong>
              <span>{source.sessionRoots.length > 0
                ? source.sessionRoots.join(", ")
                : t("remote.autoRoots")}</span>
              <span>{source.bridgeEnabled ? t("remote.stdio") : t("remote.readOnly")}</span>
            </div>
            <button type="button" onClick={() => void toggle(source)}>
              {source.enabled ? t("common.disable") : t("common.enable")}
            </button>
            <button type="button" onClick={() => void remove(source)}>{t("common.remove")}</button>
          </div>
        ))}
      </div>
      <output role="status" aria-live="polite">{status ? t(status) : ""}</output>
    </section>
  );
}
