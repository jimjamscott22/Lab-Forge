import type { AttemptView, ChallengeSummary, DashboardSummary, Step } from "../api/types";

const step = (id: number, over: Partial<Step>): Step => ({
  id,
  stepKey: `step-${id}`,
  position: id,
  kind: "task",
  title: `Step title ${id}`,
  instructionsMd: `Instructions for step ${id}`,
  completionCriteria: `Criteria ${id}`,
  hints: [],
  choices: [],
  ...over,
});

export function attemptView(options: { done?: number; status?: "in_progress" | "completed"; note?: string | null } = {}): AttemptView {
  const done = options.done ?? 0;
  const status = options.status ?? "in_progress";
  const steps: Step[] = [
    step(1, {
      title: "Find the client and the resolver",
      hints: [
        { position: 1, bodyMd: "First hint text" },
        { position: 2, bodyMd: "Second hint text" },
      ],
    }),
    step(2, { title: "Compare the two lookups" }),
    step(3, {
      kind: "check",
      title: "Choose the evidence",
      choices: [
        { id: 31, position: 1, label: "Packet 4 repeats the query" },
        { id: 32, position: 2, label: "Packet 2 says NOERROR" },
      ],
    }),
    step(4, { kind: "note", title: "Write your finding" }),
  ];
  return {
    attempt: {
      id: 7,
      challengeId: 1,
      status,
      startedAt: "2026-11-24T10:00:00.000Z",
      completedAt: status === "completed" ? "2026-11-24T10:30:00.000Z" : null,
    },
    challenge: {
      id: 1,
      slug: "dns-timeout-retry",
      version: 1,
      source: "bundled",
      title: "Read a DNS timeout and retry",
      summary: "Summary",
      difficulty: "beginner",
      objectivesMd: "Objectives",
      steps,
      evidence: [
        {
          evidenceKey: "dns-capture",
          title: "DNS capture",
          packets: [
            { no: 1, offsetMs: 0, source: "10.20.0.15:51842", destination: "10.20.0.2:53", protocol: "DNS", length: 84, info: "Standard query 0x1a2b" },
            { no: 2, offsetMs: 12, source: "10.20.0.2:53", destination: "10.20.0.15:51842", protocol: "DNS", length: 100, info: "Response 0x1a2b" },
            { no: 3, offsetMs: 4310, source: "10.20.0.15:51901", destination: "10.20.0.2:53", protocol: "DNS", length: 81, info: "Standard query 0x3c4d" },
            { no: 4, offsetMs: 6312, source: "10.20.0.15:51901", destination: "10.20.0.2:53", protocol: "DNS", length: 81, info: "Standard query 0x3c4d" },
          ],
        },
      ],
    },
    steps: steps.map((s, i) => ({
      stepId: s.id,
      status: i < done ? "completed" : "not_started",
      completedAt: i < done ? "2026-11-24T10:10:00.000Z" : null,
      selectedChoiceId: null,
    })),
    note:
      options.note == null
        ? null
        : { id: 1, attemptId: 7, bodyMd: options.note, createdAt: "2026-11-24T10:05:00.000Z", updatedAt: "2026-11-24T10:05:00.000Z" },
  };
}

export const challengeSummary = (over: Partial<ChallengeSummary> = {}): ChallengeSummary => ({
  id: 1,
  slug: "dns-timeout-retry",
  version: 1,
  source: "bundled",
  title: "Read a DNS timeout and retry",
  summary: "Find the client and resolver.",
  difficulty: "beginner",
  stepCount: 4,
  attemptCount: 0,
  latestAttemptStatus: null,
  latestAttemptId: null,
  ...over,
});

export const dashboard = (over: Partial<DashboardSummary> = {}): DashboardSummary => ({
  current: null,
  challengesAvailable: 1,
  challengesCompleted: 0,
  attemptsCompleted: 0,
  recent: [],
  ...over,
});
