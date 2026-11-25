// Mirrors src-tauri/src/models. Rust serializes with camelCase fields and snake_case enum values.

export type Difficulty = "beginner" | "intermediate" | "advanced";
export type ChallengeSource = "bundled" | "custom";
export type StepKind = "task" | "check" | "note";
export type AttemptStatus = "in_progress" | "completed";
export type StepStatus = "not_started" | "completed";

export interface ChallengeSummary {
  id: number;
  slug: string;
  version: number;
  source: ChallengeSource;
  title: string;
  summary: string;
  difficulty: Difficulty;
  stepCount: number;
  attemptCount: number;
  latestAttemptStatus: AttemptStatus | null;
  latestAttemptId: number | null;
}

export interface Hint {
  position: number;
  bodyMd: string;
}

export interface Choice {
  id: number;
  position: number;
  label: string;
}

export interface Step {
  id: number;
  stepKey: string;
  position: number;
  kind: StepKind;
  title: string;
  instructionsMd: string;
  completionCriteria: string;
  hints: Hint[];
  choices: Choice[];
}

export interface Packet {
  no: number;
  offsetMs: number;
  source: string;
  destination: string;
  protocol: string;
  length: number;
  info: string;
}

export interface Evidence {
  evidenceKey: string;
  title: string;
  packets: Packet[];
}

export interface ChallengeDetail {
  id: number;
  slug: string;
  version: number;
  source: ChallengeSource;
  title: string;
  summary: string;
  difficulty: Difficulty;
  objectivesMd: string;
  steps: Step[];
  evidence: Evidence[];
}

export interface Attempt {
  id: number;
  challengeId: number;
  status: AttemptStatus;
  startedAt: string;
  completedAt: string | null;
}

export interface AttemptStep {
  stepId: number;
  status: StepStatus;
  completedAt: string | null;
  selectedChoiceId: number | null;
}

export interface Note {
  id: number;
  attemptId: number;
  bodyMd: string;
  createdAt: string;
  updatedAt: string;
}

export interface AttemptView {
  attempt: Attempt;
  challenge: ChallengeDetail;
  steps: AttemptStep[];
  note: Note | null;
}

export interface StepOutcome {
  correct: boolean;
  explanation: string | null;
  attempt: AttemptView;
}

export interface AttemptCard {
  attemptId: number;
  challengeId: number;
  challengeTitle: string;
  status: AttemptStatus;
  startedAt: string;
  completedAt: string | null;
  stepsCompleted: number;
  stepsTotal: number;
}

export interface DashboardSummary {
  current: AttemptCard | null;
  challengesAvailable: number;
  challengesCompleted: number;
  attemptsCompleted: number;
  recent: AttemptCard[];
}

/** Input for creating or replacing a custom challenge. Every step is a plain task. */
export interface CustomChallengeInput {
  title: string;
  summary: string;
  difficulty: Difficulty;
  objectivesMd: string;
  steps: CustomStepInput[];
}

export interface CustomStepInput {
  title: string;
  instructionsMd: string;
  completionCriteria: string;
  hint?: string | null;
}
