import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppError } from "../api/client";
import { attemptCard } from "../test/fixtures";

const mocks = vi.hoisted(() => ({ listAttempts: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api/client")>();
  return { ...original, api: { ...original.api, ...mocks } };
});

import { History, HISTORY_LIMIT } from "./History";

beforeEach(() => mocks.listAttempts.mockReset());

const open = attemptCard({
  attemptId: 9,
  status: "in_progress",
  completedAt: null,
  stepsCompleted: 1,
  startedAt: "2026-11-25T10:00:00.000Z",
});
const done = attemptCard({ attemptId: 7, challengeVersion: 2 });

describe("History", () => {
  it("lists attempts newest first and opens one", async () => {
    mocks.listAttempts.mockResolvedValue([open, done]);
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<History navigate={navigate} />);
    expect(await screen.findByRole("heading", { name: "Attempt history" })).toBeInTheDocument();
    const [first, second] = screen.getAllByRole("listitem");
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
    expect(within(first!).getByText("In progress, 1 of 4 steps done")).toBeInTheDocument();
    expect(within(second!).getByText(/^Completed /)).toBeInTheDocument();
    expect(within(second!).getByText(/Version 2/)).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", {
        name: "Open attempt: Read a DNS timeout and retry, In progress, 1 of 4 steps done",
      }),
    );
    expect(navigate).toHaveBeenCalledWith({ name: "attempt", attemptId: 9 });
  });

  it("filters to one challenge and offers all attempts", async () => {
    mocks.listAttempts.mockResolvedValue([done]);
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<History challengeId={1} navigate={navigate} />);
    expect(
      await screen.findByRole("heading", { name: "Attempt history: Read a DNS timeout and retry" }),
    ).toBeInTheDocument();
    expect(mocks.listAttempts).toHaveBeenCalledWith(1);
    await user.click(screen.getByRole("button", { name: "Show all attempts" }));
    expect(navigate).toHaveBeenCalledWith({ name: "history" });
  });

  it("explains an empty history", async () => {
    mocks.listAttempts.mockResolvedValue([]);
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<History navigate={navigate} />);
    expect(await screen.findByText("You have not started any attempts yet.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Browse challenges" }));
    expect(navigate).toHaveBeenCalledWith({ name: "challenges" });
  });

  it("shows the error and a way back to all attempts", async () => {
    // Queue one rejection per load; a persistent mockRejectedValue trips Vitest's
    // unhandled-rejection tracking even though useLoad catches every call.
    const notFound = new AppError("not_found", "Challenge not found.");
    mocks.listAttempts.mockRejectedValueOnce(notFound).mockRejectedValueOnce(notFound);
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<History challengeId={5} navigate={navigate} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Challenge not found.");
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(mocks.listAttempts).toHaveBeenCalledTimes(2);
    expect(await screen.findByRole("alert")).toHaveTextContent("Challenge not found.");
    await user.click(screen.getByRole("button", { name: "Show all attempts" }));
    expect(navigate).toHaveBeenCalledWith({ name: "history" });
  });

  it("says when only the most recent attempts are shown", async () => {
    const many = (n: number) => Array.from({ length: n }, (_, i) => attemptCard({ attemptId: i + 1 }));
    mocks.listAttempts.mockResolvedValue(many(HISTORY_LIMIT));
    const { unmount } = render(<History navigate={vi.fn()} />);
    expect(await screen.findByText("Showing the 200 most recent attempts.")).toBeInTheDocument();
    unmount();

    mocks.listAttempts.mockResolvedValue(many(HISTORY_LIMIT - 1));
    render(<History navigate={vi.fn()} />);
    await screen.findByRole("heading", { name: "Attempt history" });
    expect(screen.queryByText("Showing the 200 most recent attempts.")).not.toBeInTheDocument();
  });
});
