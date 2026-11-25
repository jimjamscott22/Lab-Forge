import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AttemptView } from "../api/types";
import { attemptView } from "../test/fixtures";

const mocks = vi.hoisted(() => ({
  getAttempt: vi.fn(),
  completeStep: vi.fn(),
  saveNote: vi.fn(),
  startAttempt: vi.fn(),
  openExternalLink: vi.fn(),
}));

vi.mock("../api/client", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api/client")>();
  return { ...original, api: { ...original.api, ...mocks } };
});

import { AppError } from "../api/client";
import { Workspace } from "./Workspace";

const noteSaved = (bodyMd: string) => ({
  id: 1,
  attemptId: 7,
  bodyMd,
  createdAt: "2026-11-24T10:05:00.000Z",
  updatedAt: "2026-11-24T10:06:00.000Z",
});

function renderWorkspace(view: AttemptView, navigate = vi.fn()) {
  mocks.getAttempt.mockResolvedValue(view);
  const user = userEvent.setup();
  render(<Workspace attemptId={7} navigate={navigate} />);
  return { user, navigate };
}

beforeEach(() => {
  Object.values(mocks).forEach((m) => m.mockReset());
});

describe("Workspace: steps", () => {
  it("shows ordered steps with text status and only the reachable ones enabled", async () => {
    renderWorkspace(attemptView({ done: 1 }));
    const rail = await screen.findByRole("navigation", { name: "Steps" });
    const buttons = within(rail).getAllByRole("button");
    expect(buttons).toHaveLength(4);
    expect(buttons[0]).toHaveTextContent("Done");
    expect(buttons[1]).toHaveTextContent("Current");
    expect(buttons[1]).toBeEnabled();
    expect(buttons[2]).toHaveTextContent("Up next");
    expect(buttons[2]).toBeDisabled();
    expect(screen.getByRole("heading", { level: 2, name: "Compare the two lookups" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "1 of 4 steps done" })).toBeInTheDocument();
  });

  it("completes a task step and moves to the next one", async () => {
    const { user } = renderWorkspace(attemptView());
    mocks.completeStep.mockResolvedValue({ correct: true, explanation: null, attempt: attemptView({ done: 1 }) });
    await user.click(await screen.findByRole("button", { name: "Mark step done" }));
    expect(mocks.completeStep).toHaveBeenCalledWith(7, 1, null);
    expect(await screen.findByRole("heading", { level: 2, name: "Compare the two lookups" })).toBeInTheDocument();
  });

  it("shows the reason when Rust rejects a transition", async () => {
    const { user } = renderWorkspace(attemptView());
    mocks.completeStep.mockRejectedValue(new AppError("conflict", "Complete the earlier steps first."));
    await user.click(await screen.findByRole("button", { name: "Mark step done" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Complete the earlier steps first.");
    expect(screen.getByRole("heading", { level: 2, name: "Find the client and the resolver" })).toBeInTheDocument();
  });

  it("reveals hints one at a time, only on request", async () => {
    const { user } = renderWorkspace(attemptView());
    await screen.findByRole("heading", { level: 2, name: "Find the client and the resolver" });
    expect(screen.queryByText("First hint text")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show hint 1 of 2" }));
    expect(screen.getByText("First hint text")).toBeInTheDocument();
    expect(screen.queryByText("Second hint text")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show hint 2 of 2" }));
    expect(screen.getByText("Second hint text")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /show hint/i })).not.toBeInTheDocument();
  });
});

describe("Workspace: knowledge check", () => {
  it("needs a choice, explains a wrong answer, and advances on the right one", async () => {
    const { user } = renderWorkspace(attemptView({ done: 2 }));
    const submit = await screen.findByRole("button", { name: "Check answer" });
    expect(submit).toBeDisabled();

    await user.click(screen.getByRole("radio", { name: /Packet 2 says NOERROR/ }));
    mocks.completeStep.mockResolvedValueOnce({
      correct: false,
      explanation: "That answer belongs to the first lookup.",
      attempt: attemptView({ done: 2 }),
    });
    await user.click(submit);
    expect(mocks.completeStep).toHaveBeenLastCalledWith(7, 3, 32);
    expect(await screen.findByText(/Not quite\./)).toBeInTheDocument();
    expect(screen.getByText(/belongs to the first lookup/)).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: "Choose the evidence" })).toBeInTheDocument();

    await user.click(screen.getByRole("radio", { name: /Packet 4 repeats the query/ }));
    mocks.completeStep.mockResolvedValueOnce({
      correct: true,
      explanation: "Correct reasoning.",
      attempt: attemptView({ done: 3 }),
    });
    await user.click(screen.getByRole("button", { name: "Check answer" }));
    expect(mocks.completeStep).toHaveBeenLastCalledWith(7, 3, 31);
    expect(await screen.findByRole("heading", { level: 2, name: "Write your finding" })).toBeInTheDocument();
  });
});

describe("Workspace: notebook", () => {
  it("keeps the text and reports the failure when a save fails, then recovers on retry", async () => {
    const { user } = renderWorkspace(attemptView({ done: 3 }));
    const editor = await screen.findByRole("textbox", { name: "Notebook" });
    await user.type(editor, "10.20.0.15 is the client");

    mocks.saveNote.mockRejectedValueOnce(new AppError("internal", "LabForge could not read or save its data."));
    await user.click(screen.getByRole("button", { name: "Save now" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/not saved/i);
    expect(alert).toHaveTextContent(/still in the notebook/i);
    expect(editor).toHaveValue("10.20.0.15 is the client");

    mocks.saveNote.mockResolvedValueOnce(noteSaved("10.20.0.15 is the client"));
    await user.click(screen.getByRole("button", { name: "Try saving again" }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    expect(screen.getByRole("status", { name: "" })).toBeDefined();
    expect(mocks.saveNote).toHaveBeenCalledTimes(2);
    expect(mocks.saveNote).toHaveBeenLastCalledWith(7, "10.20.0.15 is the client");
  });

  it("does not complete the note step when the notebook cannot be saved", async () => {
    const { user } = renderWorkspace(attemptView({ done: 3 }));
    await user.type(await screen.findByRole("textbox", { name: "Notebook" }), "finding");
    mocks.saveNote.mockRejectedValue(new AppError("internal", "LabForge could not read or save its data."));
    await user.click(screen.getByRole("button", { name: "Complete step" }));

    expect(await screen.findAllByRole("alert")).not.toHaveLength(0);
    expect(mocks.completeStep).not.toHaveBeenCalled();
    expect(screen.getByRole("textbox", { name: "Notebook" })).toHaveValue("finding");
  });

  it("saves the draft first, then completes the note step and shows the summary", async () => {
    const { user } = renderWorkspace(attemptView({ done: 3 }));
    await user.type(await screen.findByRole("textbox", { name: "Notebook" }), "Packet 4 retried packet 3.");
    mocks.saveNote.mockResolvedValue(noteSaved("Packet 4 retried packet 3."));
    mocks.completeStep.mockResolvedValue({
      correct: true,
      explanation: null,
      attempt: attemptView({ done: 4, status: "completed", note: "Packet 4 retried packet 3." }),
    });
    await user.click(screen.getByRole("button", { name: "Complete step" }));

    expect(await screen.findByRole("heading", { name: "Attempt complete" })).toBeInTheDocument();
    expect(mocks.saveNote.mock.invocationCallOrder[0]).toBeLessThan(mocks.completeStep.mock.invocationCallOrder[0]!);
    expect(screen.getByText("Packet 4 retried packet 3.", { selector: "p" })).toBeInTheDocument();
  });

  it("previews notes without rendering raw HTML", async () => {
    const { user } = renderWorkspace(attemptView({ done: 3, note: "**client** <script>x()</script>" }));
    await user.click(await screen.findByRole("button", { name: "Preview" }));
    const preview = screen.getByRole("region", { name: "Notebook preview" });
    expect(within(preview).getByText("client").tagName).toBe("STRONG");
    expect(preview.querySelector("script")).toBeNull();
  });
});

describe("Workspace: keyboard", () => {
  it("reaches the hint control and the note editor with Tab alone", async () => {
    const { user } = renderWorkspace(attemptView());
    await screen.findByRole("heading", { level: 2, name: "Find the client and the resolver" });

    const seen: string[] = [];
    for (let i = 0; i < 20; i++) {
      await user.tab();
      const el = document.activeElement as HTMLElement;
      seen.push(el.getAttribute("aria-label") ?? el.textContent ?? el.tagName);
      if (el.id === "notebook-text") break;
    }
    expect(document.activeElement).toHaveAttribute("id", "notebook-text");
    expect(seen).toContain("Show hint 1 of 2");
    expect(seen.indexOf("Show hint 1 of 2")).toBeLessThan(seen.length - 1);
  });

  it("activates hints and completes a step with Enter and Space", async () => {
    const { user } = renderWorkspace(attemptView());
    const hint = await screen.findByRole("button", { name: "Show hint 1 of 2" });
    hint.focus();
    await user.keyboard("{Enter}");
    expect(screen.getByText("First hint text")).toBeInTheDocument();

    mocks.completeStep.mockResolvedValue({ correct: true, explanation: null, attempt: attemptView({ done: 1 }) });
    screen.getByRole("button", { name: "Mark step done" }).focus();
    await user.keyboard(" ");
    expect(mocks.completeStep).toHaveBeenCalledWith(7, 1, null);
  });

  it("moves focus to the step heading when a step changes", async () => {
    const { user } = renderWorkspace(attemptView({ done: 1 }));
    await screen.findByRole("heading", { level: 2, name: "Compare the two lookups" });
    await user.click(within(screen.getByRole("navigation", { name: "Steps" })).getAllByRole("button")[0]!);
    expect(screen.getByRole("heading", { level: 2, name: "Find the client and the resolver" })).toHaveFocus();
  });
});

describe("Workspace: focus after completing", () => {
  it("moves focus to the next step heading, and to the summary after the last step", async () => {
    const { user } = renderWorkspace(attemptView());
    mocks.completeStep.mockResolvedValueOnce({ correct: true, explanation: null, attempt: attemptView({ done: 1 }) });
    await user.click(await screen.findByRole("button", { name: "Mark step done" }));
    expect(await screen.findByRole("heading", { level: 2, name: "Compare the two lookups" })).toHaveFocus();
  });

  it("focuses the summary heading when the final step completes", async () => {
    const { user } = renderWorkspace(attemptView({ done: 3 }));
    await user.type(await screen.findByRole("textbox", { name: "Notebook" }), "done");
    mocks.saveNote.mockResolvedValue(noteSaved("done"));
    mocks.completeStep.mockResolvedValue({
      correct: true,
      explanation: null,
      attempt: attemptView({ done: 4, status: "completed", note: "done" }),
    });
    await user.click(screen.getByRole("button", { name: "Complete step" }));
    expect(await screen.findByRole("heading", { name: "Attempt complete" })).toHaveFocus();
  });
});

describe("Workspace: finished attempt", () => {
  it("shows the summary with notes and starts another attempt", async () => {
    const navigate = vi.fn();
    const { user } = renderWorkspace(attemptView({ done: 4, status: "completed", note: "My **finding**" }), navigate);
    expect(await screen.findByRole("heading", { name: "Attempt complete" })).toBeInTheDocument();
    expect(screen.getByText("finding").tagName).toBe("STRONG");

    mocks.startAttempt.mockResolvedValue({ ...attemptView(), attempt: { ...attemptView().attempt, id: 8 } });
    await user.click(screen.getByRole("button", { name: "Start another attempt" }));
    expect(mocks.startAttempt).toHaveBeenCalledWith(1);
    expect(navigate).toHaveBeenCalledWith({ name: "attempt", attemptId: 8 });
  });

  it("reports why another attempt could not start", async () => {
    const { user } = renderWorkspace(attemptView({ done: 4, status: "completed", note: "x" }));
    mocks.startAttempt.mockRejectedValue(new AppError("conflict", "This challenge already has an attempt in progress. Resume it instead."));
    await user.click(await screen.findByRole("button", { name: "Start another attempt" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("already has an attempt in progress");
  });
});

describe("Workspace: loading", () => {
  it("offers a retry when the attempt cannot be loaded", async () => {
    mocks.getAttempt.mockRejectedValueOnce(new AppError("internal", "LabForge could not read or save its data."));
    mocks.getAttempt.mockResolvedValueOnce(attemptView());
    const user = userEvent.setup();
    render(<Workspace attemptId={7} navigate={vi.fn()} />);
    expect(await screen.findByRole("alert")).toHaveTextContent(/could not read or save/);
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByRole("heading", { level: 1, name: "Read a DNS timeout and retry" })).toBeInTheDocument();
  });
});
