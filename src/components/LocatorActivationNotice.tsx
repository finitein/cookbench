import type { LocatorActivationResult } from "../services/locator";

export function LocatorActivationNotice({ result }: { result: LocatorActivationResult | null }) {
  if (!result) return null;

  if (result.status === "focused") {
    return <output role="status" aria-live="polite">Returned to the original work surface.</output>;
  }

  if (result.status === "visibleFallback" && result.resumeSessionId) {
    return (
      <output role="status" aria-live="polite">
        Original surface unavailable. Resume session: <code>{result.resumeSessionId}</code>
      </output>
    );
  }

  return <output role="status" aria-live="polite">Original work surface is unavailable.</output>;
}
