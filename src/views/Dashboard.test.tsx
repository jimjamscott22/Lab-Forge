import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { dashboard } from "../test/fixtures";

const mocks = vi.hoisted(() => ({ getDashboard: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api/client")>();
  return { ...original, api: { ...original.api, ...mocks } };
});

import { Dashboard } from "./Dashboard";

beforeEach(() => mocks.getDashboard.mockReset());

const card = {
  attemptId: 7,
  challengeId: 1,
  challengeTitle: "Read a DNS timeout and retry",
  challengeVersion: 1,
  status: "in_progress" as const,
  startedAt: "2026-11-24T10:00:00.000Z",
  completedAt: null,
  stepsCompleted: 2,
  stepsTotal: 4,
};

describe("Dashboard", () => {
  it("offers Resume for the open attempt, with progress in text", async () => {
    mocks.getDashboard.mockResolvedValue(dashboard({ current: card, recent: [card] }));
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<Dashboard navigate={navigate} />);
    expect(await screen.findByRole("heading", { name: "Continue your attempt" })).toBeInTheDocument();
    expect(screen.getAllByText("2 of 4 steps done").length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Resume" }));
    expect(navigate).toHaveBeenCalledWith({ name: "attempt", attemptId: 7 });
  });

  it("invites the learner to browse challenges when nothing is open", async () => {
    mocks.getDashboard.mockResolvedValue(dashboard());
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<Dashboard navigate={navigate} />);
    expect(await screen.findByText(/no attempt in progress/i)).toBeInTheDocument();
    expect(screen.getByText("0 of 1 challenge completed, 0 attempts finished.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Browse challenges" }));
    expect(navigate).toHaveBeenCalledWith({ name: "challenges" });
  });

  it("summarises finished work", async () => {
    mocks.getDashboard.mockResolvedValue(
      dashboard({
        challengesCompleted: 1,
        attemptsCompleted: 2,
        recent: [{ ...card, status: "completed", completedAt: "2026-11-24T10:30:00.000Z", stepsCompleted: 4 }],
      }),
    );
    render(<Dashboard navigate={vi.fn()} />);
    expect(await screen.findByText("1 of 1 challenge completed, 2 attempts finished.")).toBeInTheDocument();
    expect(screen.getByText(/^Completed /)).toBeInTheDocument();
  });
});
