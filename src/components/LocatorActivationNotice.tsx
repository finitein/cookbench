import { useEffect, useState } from "react";

import type { LocatorActivationResult } from "../services/locator";

export function LocatorActivationNotice({ result }: { result: LocatorActivationResult | null }) {
  const [expiredResult, setExpiredResult] = useState<LocatorActivationResult | null>(null);

  useEffect(() => {
    if (!result || result.status === "focused") return;
    const timeout = window.setTimeout(() => setExpiredResult(result), 20_000);
    return () => window.clearTimeout(timeout);
  }, [result]);

  if (!result || result === expiredResult) return null;

  if (result.status === "focused") {
    return null;
  }

  if (result.target === "exactThread" && result.status === "visibleFallback") {
    return <output role="status" aria-live="polite">Opening the matching Codex task.</output>;
  }

  if (result.status === "visibleFallback" && result.resumeSessionId) {
    return (
      <output role="status" aria-live="polite">
        Cookbench could not open the original session. It has kept the Stove visible here; use this session ID in your original tool: <code>{result.resumeSessionId}</code>
      </output>
    );
  }

  return <output role="status" aria-live="polite">Cookbench could not open the original session. It has kept the Stove visible here.</output>;
}
