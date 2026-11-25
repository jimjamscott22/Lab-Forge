import { invoke } from "@tauri-apps/api/core";
import type {
  AttemptView,
  ChallengeDetail,
  ChallengeSummary,
  CustomChallengeInput,
  DashboardSummary,
  Note,
  StepOutcome,
} from "./types";

/** Error shape produced by the Rust `AppError` serializer: `{ code, message }`. */
export type ErrorCode = "validation" | "not_found" | "conflict" | "internal";

export class AppError extends Error {
  readonly code: ErrorCode;
  constructor(code: ErrorCode, message: string) {
    super(message);
    this.name = "AppError";
    this.code = code;
  }
}

const GENERIC_MESSAGE =
  "LabForge could not complete that request. Try again, and restart the app if it keeps happening.";

export function toAppError(raw: unknown): AppError {
  if (raw instanceof AppError) return raw;
  if (typeof raw === "object" && raw !== null && "message" in raw) {
    const { code, message } = raw as { code?: unknown; message?: unknown };
    if (typeof message === "string") {
      const known = ["validation", "not_found", "conflict", "internal"];
      return new AppError(known.includes(code as string) ? (code as ErrorCode) : "internal", message);
    }
  }
  // Plain strings or anything unexpected: never surface raw internals.
  return new AppError("internal", GENERIC_MESSAGE);
}

/**
 * The complete contract between React and Rust. Adding a command means adding it here,
 * in `src-tauri/build.rs`, and in `src-tauri/capabilities/default.json`.
 * Argument names are camelCase; Tauri maps them to the Rust snake_case parameters.
 */
interface Commands {
  list_challenges: { args: { query?: string | null }; result: ChallengeSummary[] };
  get_challenge: { args: { challengeId: number }; result: ChallengeDetail };
  create_challenge: { args: { input: CustomChallengeInput }; result: ChallengeDetail };
  update_challenge: {
    args: { challengeId: number; input: CustomChallengeInput };
    result: ChallengeDetail;
  };
  delete_challenge: { args: { challengeId: number }; result: void };
  start_attempt: { args: { challengeId: number }; result: AttemptView };
  get_attempt: { args: { attemptId: number }; result: AttemptView };
  complete_step: {
    args: { attemptId: number; stepId: number; choiceId: number | null };
    result: StepOutcome;
  };
  save_note: { args: { attemptId: number; bodyMd: string }; result: Note };
  get_dashboard: { args: Record<string, never>; result: DashboardSummary };
  open_external_link: { args: { url: string }; result: void };
}

async function call<K extends keyof Commands>(
  command: K,
  args: Commands[K]["args"],
): Promise<Commands[K]["result"]> {
  try {
    return await invoke<Commands[K]["result"]>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

export const api = {
  listChallenges: (query?: string) => call("list_challenges", { query: query ?? null }),
  getChallenge: (challengeId: number) => call("get_challenge", { challengeId }),
  createChallenge: (input: CustomChallengeInput) => call("create_challenge", { input }),
  updateChallenge: (challengeId: number, input: CustomChallengeInput) =>
    call("update_challenge", { challengeId, input }),
  deleteChallenge: (challengeId: number) => call("delete_challenge", { challengeId }),
  startAttempt: (challengeId: number) => call("start_attempt", { challengeId }),
  getAttempt: (attemptId: number) => call("get_attempt", { attemptId }),
  completeStep: (attemptId: number, stepId: number, choiceId: number | null = null) =>
    call("complete_step", { attemptId, stepId, choiceId }),
  saveNote: (attemptId: number, bodyMd: string) => call("save_note", { attemptId, bodyMd }),
  getDashboard: () => call("get_dashboard", {}),
  openExternalLink: (url: string) => call("open_external_link", { url }),
};
