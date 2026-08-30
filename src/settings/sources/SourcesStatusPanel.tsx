import { useEffect, useState } from "react";

import { getLocalSourceStatus, type LocalSourceStatus } from "./service";
import "./source-status.css";

const HEALTH_LABEL: Record<LocalSourceStatus["health"], string> = {
  healthy: "Watching",
  degraded: "Needs attention",
  unavailable: "Unavailable",
};

function sessionLabel(count: number) {
  return `${count} session${count === 1 ? "" : "s"}`;
}

function errorLabel(count: number) {
  return `${count} parsing issue${count === 1 ? "" : "s"}`;
}

export function SourcesStatusPanel() {
  const [sources, setSources] = useState<LocalSourceStatus[]>([]);
  const [status, setStatus] = useState("");

  useEffect(() => {
    void getLocalSourceStatus().then((response) => {
      setSources(response.sources);
    }).catch(() => {
      setStatus("Local source status is unavailable.");
    });
  }, []);

  return (
    <section className="source-status" aria-labelledby="source-status-title">
      <header>
        <h2 id="source-status-title">Local Sources</h2>
        <p>Read-only session discovery on this computer.</p>
      </header>
      <ul className="source-status__list" aria-label="Local source status">
        {sources.map((source) => (
          <li key={source.harness} className="source-status__source">
            <div className="source-status__identity">
              <strong>{source.label}</strong>
              <span title={source.rootDisplay}>{source.rootDisplay}</span>
            </div>
            <div className="source-status__details">
              <span className={`source-status__health source-status__health--${source.health}`}>
                {HEALTH_LABEL[source.health]}
              </span>
              <span>{sessionLabel(source.discoveredSessions)}</span>
              {source.parserErrors > 0 ? <span>{errorLabel(source.parserErrors)}</span> : null}
            </div>
          </li>
        ))}
      </ul>
      <output role="status" aria-live="polite">{status}</output>
    </section>
  );
}
