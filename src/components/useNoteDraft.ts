import { useCallback, useEffect, useRef, useState } from "react";
import { api, toAppError } from "../api/client";

export type SaveState =
  | { kind: "saved"; at: string | null }
  | { kind: "dirty" }
  | { kind: "saving" }
  | { kind: "error"; message: string };

/**
 * Local draft of an attempt's note with debounced autosave.
 * A failed save never discards text: the draft stays in the editor and the state
 * becomes `error` until a later save succeeds.
 */
export function useNoteDraft(
  attemptId: number,
  initialBody: string,
  initialSavedAt: string | null,
  debounceMs = 800,
) {
  const [text, setTextState] = useState(initialBody);
  const [state, setState] = useState<SaveState>({ kind: "saved", at: initialSavedAt });

  const textRef = useRef(initialBody);
  const savedRef = useRef(initialBody);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const failed = useRef(false);

  const clearTimer = () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
  };

  /** Resolves true when the latest text is stored, false if the save failed. */
  const save = useCallback((): Promise<boolean> => {
    clearTimer();
    const run = async (): Promise<boolean> => {
      const body = textRef.current;
      if (body === savedRef.current && !failed.current) return true;
      setState({ kind: "saving" });
      try {
        const note = await api.saveNote(attemptId, body);
        savedRef.current = body;
        failed.current = false;
        // If the learner kept typing during the save, stay dirty; autosave will follow.
        setState(
          textRef.current === body ? { kind: "saved", at: note.updatedAt } : { kind: "dirty" },
        );
        return true;
      } catch (e) {
        failed.current = true;
        setState({ kind: "error", message: toAppError(e).message });
        return false;
      }
    };
    const result = queue.current.then(run, run);
    queue.current = result;
    return result;
  }, [attemptId]);

  const setText = useCallback(
    (value: string) => {
      textRef.current = value;
      setTextState(value);
      setState({ kind: "dirty" });
      clearTimer();
      timer.current = setTimeout(() => void save(), debounceMs);
    },
    [debounceMs, save],
  );

  // Persist pending edits when leaving the workspace.
  useEffect(
    () => () => {
      if (timer.current) {
        clearTimeout(timer.current);
        timer.current = null;
        if (textRef.current !== savedRef.current) void api.saveNote(attemptId, textRef.current).catch(() => undefined);
      }
    },
    [attemptId],
  );

  return { text, setText, state, save };
}
