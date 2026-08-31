import { invoke } from "@tauri-apps/api/core";

export type LocalSourceHarness = "codex" | "claudeCode" | "pi" | (string & {});
export type LocalSourceHealth = "healthy" | "degraded" | "unavailable";
export type LocalSourceSupportTier = "full" | "standard" | "experimental";
export type LocalSourceObservation = "nativeSessions" | "structuredHook" | "presenceOnly";

export type LocalSourceStatus = {
  harness: LocalSourceHarness;
  label: string;
  tier: LocalSourceSupportTier;
  observation: LocalSourceObservation;
  health: LocalSourceHealth;
  rootDisplay: string;
  discoveredSessions: number;
  parserErrors: number;
};

export type LocalSourceStatusResponse = {
  sources: LocalSourceStatus[];
};

export function getLocalSourceStatus(): Promise<LocalSourceStatusResponse> {
  return invoke<LocalSourceStatusResponse>("get_local_source_status");
}
