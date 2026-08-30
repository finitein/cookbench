import { invoke } from "@tauri-apps/api/core";

export type LocatorActivationTarget =
  | "exactPane"
  | "exactThread"
  | "applicationWindow"
  | "projectDirectory"
  | "resumeInstructions"
  | "unavailable";

export type LocatorActivationStatus = "focused" | "visibleFallback" | "unavailable";

export type LocatorActivationResult = {
  target: LocatorActivationTarget;
  status: LocatorActivationStatus;
  resumeSessionId: string | null;
};

export type LocatorTransport = {
  activate: (stoveId: string) => Promise<LocatorActivationResult>;
};

export const tauriLocatorTransport: LocatorTransport = {
  activate: (stoveId) => invoke<LocatorActivationResult>("activate_stove_locator", { stoveId }),
};

export async function activateStove(
  stoveId: string,
  transport: LocatorTransport = tauriLocatorTransport,
): Promise<LocatorActivationResult> {
  return transport.activate(stoveId);
}
