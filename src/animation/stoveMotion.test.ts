import { describe, expect, it } from "vitest";

import { motionForStoveTransition, settleCompletionEffect } from "./stoveMotion";

const defaults = { reducedMotion: false, soundEnabled: false };

describe("motionForStoveTransition", () => {
  it("emits one restrained effect for a new Cooked transition", () => {
    const result = motionForStoveTransition(
      { previousState: "cooking", nextState: "cooked" },
      defaults,
    );

    expect(result).toMatchObject({
      completion: "finishing",
      notify: true,
      playSound: false,
      ring: "complete",
      rotates: false,
    });
  });

  it("does not replay a completion effect for a stale Cooked snapshot", () => {
    const result = motionForStoveTransition(
      { nextState: "cooked", isInitialSnapshot: true },
      defaults,
    );

    expect(result).toMatchObject({ completion: "settled", notify: false, playSound: false });
  });

  it("does not replay a completion effect when Cooked is received again", () => {
    const result = motionForStoveTransition(
      { previousState: "cooked", nextState: "cooked" },
      defaults,
    );

    expect(result).toMatchObject({ completion: "settled", notify: false, playSound: false });
  });

  it("plays sound only when the user enables it", () => {
    const result = motionForStoveTransition(
      { previousState: "cooking", nextState: "cooked" },
      { ...defaults, soundEnabled: true },
    );

    expect(result.playSound).toBe(true);
  });

  it("settles immediately with reduced motion", () => {
    const result = motionForStoveTransition(
      { previousState: "cooking", nextState: "cooked" },
      { ...defaults, reducedMotion: true },
    );

    expect(result).toMatchObject({ completion: "settled", rotates: false, ring: "complete" });
  });

  it("settles after the finish effect as a static complete ring", () => {
    const initial = motionForStoveTransition(
      { previousState: "cooking", nextState: "cooked" },
      defaults,
    );

    expect(settleCompletionEffect(initial)).toMatchObject({
      completion: "settled",
      ring: "complete",
      rotates: false,
      notify: false,
      playSound: false,
    });
  });
});
