import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { challengeSummary } from "../test/fixtures";

const mocks = vi.hoisted(() => ({ listChallenges: vi.fn(), startAttempt: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api/client")>();
  return { ...original, api: { ...original.api, ...mocks } };
});

import { AppError } from "../api/client";
import { Challenges } from "./Challenges";

beforeEach(() => Object.values(mocks).forEach((m) => m.mockReset()));

describe("Challenges", () => {
  it("lists challenges with status text and a Start action", async () => {
    mocks.listChallenges.mockResolvedValue([challengeSummary()]);
    render(<Challenges navigate={vi.fn()} />);
    expect(await screen.findByRole("heading", { name: "Read a DNS timeout and retry" })).toBeInTheDocument();
    expect(screen.getByText(/Beginner difficulty, 4 steps\. Not started\./)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start: Read a DNS timeout and retry" })).toBeInTheDocument();
  });

  it("searches through Rust after a short pause", async () => {
    mocks.listChallenges.mockResolvedValue([challengeSummary()]);
    const user = userEvent.setup();
    render(<Challenges navigate={vi.fn()} />);
    await screen.findByRole("heading", { name: "Read a DNS timeout and retry" });

    mocks.listChallenges.mockResolvedValue([]);
    await user.type(screen.getByRole("searchbox", { name: "Search challenges" }), "xyz");
    await waitFor(() => expect(mocks.listChallenges).toHaveBeenLastCalledWith("xyz"));
    expect(await screen.findByText(/No challenges match “xyz”/)).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("0 challenges");
  });

  it("offers Resume for an open attempt and navigates to it", async () => {
    mocks.listChallenges.mockResolvedValue([challengeSummary({ attemptCount: 1, latestAttemptStatus: "in_progress", latestAttemptId: 9 })]);
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<Challenges navigate={navigate} />);
    await user.click(await screen.findByRole("button", { name: /^Resume/ }));
    expect(navigate).toHaveBeenCalledWith({ name: "attempt", attemptId: 9 });
    expect(mocks.startAttempt).not.toHaveBeenCalled();
  });

  it("offers a retry and a review after completion", async () => {
    mocks.listChallenges.mockResolvedValue([challengeSummary({ attemptCount: 2, latestAttemptStatus: "completed", latestAttemptId: 4 })]);
    const navigate = vi.fn();
    const user = userEvent.setup();
    render(<Challenges navigate={navigate} />);
    expect(await screen.findByText(/Completed\. 2 attempts so far\./)).toBeInTheDocument();
    mocks.startAttempt.mockResolvedValue({ attempt: { id: 10 } });
    await user.click(screen.getByRole("button", { name: /^Start again/ }));
    expect(mocks.startAttempt).toHaveBeenCalledWith(1);
    expect(navigate).toHaveBeenCalledWith({ name: "attempt", attemptId: 10 });
    await user.click(screen.getByRole("button", { name: /^Review last attempt/ }));
    expect(navigate).toHaveBeenLastCalledWith({ name: "attempt", attemptId: 4 });
  });

  it("shows the reason when an attempt cannot start", async () => {
    mocks.listChallenges.mockResolvedValue([challengeSummary()]);
    mocks.startAttempt.mockRejectedValue(new AppError("conflict", "This challenge already has an attempt in progress. Resume it instead."));
    const user = userEvent.setup();
    render(<Challenges navigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: /^Start:/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Resume it instead");
  });

  it("shows a retry when loading fails", async () => {
    mocks.listChallenges.mockRejectedValueOnce(new AppError("internal", "LabForge could not read or save its data."));
    mocks.listChallenges.mockResolvedValueOnce([challengeSummary()]);
    const user = userEvent.setup();
    render(<Challenges navigate={vi.fn()} />);
    expect(await screen.findByRole("alert")).toHaveTextContent(/could not read/);
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByRole("heading", { name: "Read a DNS timeout and retry" })).toBeInTheDocument();
  });
});
