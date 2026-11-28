import { beforeEach, describe, expect, it, vi } from "vitest";

// A plain function stands in for the bridge. vi.fn() would track the rejected promise it returns.
type Impl = (command: string, args: unknown) => Promise<unknown>;
const bridge = vi.hoisted(() => ({
  impl: (async () => undefined) as Impl,
  calls: [] as [string, unknown][],
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args: unknown) => {
    bridge.calls.push([command, args]);
    return bridge.impl(command, args);
  },
}));
const lastCall = () => bridge.calls[bridge.calls.length - 1];

import { AppError, api, toAppError } from "./client";

beforeEach(() => {
  bridge.calls.length = 0;
  bridge.impl = async () => undefined;
});

describe("toAppError", () => {
  it("keeps the code and message Rust sends", () => {
    const e = toAppError({ code: "conflict", message: "Complete the earlier steps first." });
    expect(e).toBeInstanceOf(AppError);
    expect(e.code).toBe("conflict");
    expect(e.message).toBe("Complete the earlier steps first.");
  });

  it("downgrades unknown codes to internal but keeps a message that Rust wrote", () => {
    expect(toAppError({ code: "weird", message: "Hi" }).code).toBe("internal");
  });

  it("never shows raw strings or unexpected values from the bridge", () => {
    for (const raw of ["command list_challenges not allowed by ACL", 42, null, undefined, {}]) {
      const e = toAppError(raw);
      expect(e.code).toBe("internal");
      expect(e.message).not.toContain("ACL");
      expect(e.message).toMatch(/could not complete/i);
    }
  });
});

describe("api wrapper", () => {
  it("calls commands with the camelCase argument names Tauri expects", async () => {
    await api.completeStep(7, 3, 31);
    expect(lastCall()).toEqual(["complete_step", { attemptId: 7, stepId: 3, choiceId: 31 }]);
    await api.saveNote(7, "text");
    expect(lastCall()).toEqual(["save_note", { attemptId: 7, bodyMd: "text" }]);
    await api.listChallenges("dns");
    expect(lastCall()).toEqual(["list_challenges", { query: "dns" }]);
    await api.listChallenges();
    expect(lastCall()).toEqual(["list_challenges", { query: null }]);
  });

  it("turns a rejected invoke into an AppError", async () => {
    bridge.impl = () =>
      Promise.reject({ code: "validation", message: "Notes can be up to 100000 characters." });
    const error = await api.saveNote(1, "x").catch((e: unknown) => e);
    expect(error).toBeInstanceOf(AppError);
    expect((error as AppError).code).toBe("validation");
    expect((error as AppError).message).toBe("Notes can be up to 100000 characters.");
  });
});

describe("listAttempts", () => {
  it("sends a null challengeId when unfiltered", async () => {
    bridge.impl = async () => [];
    await api.listAttempts();
    expect(lastCall()).toEqual(["list_attempts", { challengeId: null }]);
    await api.listAttempts(3);
    expect(lastCall()).toEqual(["list_attempts", { challengeId: 3 }]);
  });
});
