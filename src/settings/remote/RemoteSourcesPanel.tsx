import { useEffect, useState } from "react";

import {
  configureRemoteSource,
  getRemoteSources,
  removeRemoteSource,
  type RemoteSourceWire,
} from "./service";

export function RemoteSourcesPanel() {
  const [sources, setSources] = useState<RemoteSourceWire[]>([]);
  const [alias, setAlias] = useState("");
  const [roots, setRoots] = useState("");
  const [bridgeEnabled, setBridgeEnabled] = useState(false);
  const [bridgeBinaryPath, setBridgeBinaryPath] = useState("");
  const [status, setStatus] = useState("");

  useEffect(() => {
    void getRemoteSources().then(setSources).catch(() => setStatus("SSH sources are unavailable."));
  }, []);

  const add = async () => {
    setStatus("");
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
      setStatus("SSH source saved.");
    } catch {
      setStatus("SSH source could not be saved.");
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
      setStatus("SSH source could not be updated.");
    }
  };

  const remove = async (source: RemoteSourceWire) => {
    try {
      setSources(await removeRemoteSource(source.id));
      setStatus("SSH source removed.");
    } catch {
      setStatus("SSH source could not be removed.");
    }
  };

  return (
    <section className="remote-settings" aria-labelledby="remote-settings-title">
      <h2 id="remote-settings-title">SSH Sources</h2>
      <div className="remote-settings__add">
        <label>
          <span>SSH alias</span>
          <input value={alias} onChange={(event) => setAlias(event.target.value)} autoComplete="off" />
        </label>
        <label className="remote-settings__bridge-choice">
          <input
            type="checkbox"
            checked={bridgeEnabled}
            onChange={(event) => setBridgeEnabled(event.target.checked)}
          />
          <span>Temporary bridge over SSH stdio</span>
        </label>
        {bridgeEnabled ? (
          <label>
            <span>Compatible bridge binary (optional)</span>
            <input
              value={bridgeBinaryPath}
              onChange={(event) => setBridgeBinaryPath(event.target.value)}
              autoComplete="off"
              placeholder="Packaged binary"
            />
          </label>
        ) : null}
        <label>
          <span>Session roots</span>
          <input
            value={roots}
            onChange={(event) => setRoots(event.target.value)}
            autoComplete="off"
            placeholder="Automatic when empty"
          />
        </label>
        <button type="button" disabled={!alias.trim()} onClick={() => void add()}>Add</button>
      </div>
      <div className="remote-settings__sources">
        {sources.map((source) => (
          <div key={source.id} className="remote-settings__source">
            <div>
              <strong>{source.alias}</strong>
              <span>{source.sessionRoots.length > 0
                ? source.sessionRoots.join(", ")
                : "Automatic supported Harness roots"}</span>
              <span>{source.bridgeEnabled ? "Temporary stdio bridge" : "Zero-install read-only"}</span>
            </div>
            <button type="button" onClick={() => void toggle(source)}>
              {source.enabled ? "Disable" : "Enable"}
            </button>
            <button type="button" onClick={() => void remove(source)}>Remove</button>
          </div>
        ))}
      </div>
      <output role="status" aria-live="polite">{status}</output>
    </section>
  );
}
