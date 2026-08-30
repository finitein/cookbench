import { invoke } from "@tauri-apps/api/core";

export type LocalSourceHarness = "codex" | "claudeCode" | "pi";
export type LocalSourceHealth = "healthy" | "degraded" | "unavailable";

export type LocalSourceStatus = {
  harness: LocalSourceHarness;
  label: string;
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
