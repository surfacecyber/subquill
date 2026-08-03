import { useCallback, useEffect, useRef, useState } from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";

import {
  cancelJob,
  getJobResult,
  getJobStatus,
  listenJobProgress,
  startNoteJob,
} from "../api/job";
import type { AppErrorPayload } from "../types/settings";
import type {
  BatchItemView,
  JobProgress,
  JobProgressStage,
  JobResult,
  JobStatus,
} from "../types/job";

const POLL_INTERVAL_MS = 2000;

const TERMINAL_STAGES: ReadonlySet<JobProgressStage> = new Set([
  "done",
  "failed",
  "cancelled",
]);

function isTerminalStage(stage: JobProgressStage): boolean {
  return TERMINAL_STAGES.has(stage);
}

function isActiveStatus(status: JobStatus): boolean {
  return status === "queued" || status === "running";
}

function terminalStatusFromStage(stage: JobProgressStage): JobStatus {
  if (stage === "done") {
    return "completed";
  }
  if (stage === "cancelled") {
    return "cancelled";
  }
  return "failed";
}

export function useNoteJob() {
  const [jobId, setJobId] = useState<string | null>(null);
  const [status, setStatus] = useState<JobStatus | null>(null);
  const [progress, setProgress] = useState<JobProgress | null>(null);
  const [result, setResult] = useState<JobResult | null>(null);
  const [error, setError] = useState<AppErrorPayload | null>(null);
  const [batchItems, setBatchItems] = useState<BatchItemView[]>([]);
  const [isActive, setIsActive] = useState(false);
  const [cancelRequested, setCancelRequested] = useState(false);

  const unlistenRef = useRef<UnlistenFn | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const jobIdRef = useRef<string | null>(null);
  const pendingEventsRef = useRef<JobProgress[]>([]);
  const isTerminalRef = useRef(false);
  /** Prevents concurrent start() while listener/claim is in flight. */
  const startInFlightRef = useRef(false);

  const cleanupListener = useCallback(() => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  const stopPolling = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  const handleTerminal = useCallback(
    async (id: string, terminalStatus: JobStatus, terminalError?: AppErrorPayload) => {
      isTerminalRef.current = true;
      startInFlightRef.current = false;
      stopPolling();
      setIsActive(false);
      setStatus(terminalStatus);

      // Completed always has a primary result; cancelled/failed may keep partial success.
      if (
        terminalStatus === "completed" ||
        terminalStatus === "cancelled" ||
        terminalStatus === "failed"
      ) {
        try {
          const jobResult = await getJobResult(id);
          setResult(jobResult);
          // Keep failure banner when Failed; cancel/complete rely on batch summary.
          setError(terminalStatus === "failed" ? (terminalError ?? null) : null);
        } catch (err) {
          setResult(null);
          setError((err as AppErrorPayload) ?? terminalError ?? null);
        }

        try {
          const response = await getJobStatus(id);
          setBatchItems(response.batch_items ?? []);
        } catch {
          // Best-effort; progress UI already has item_index/total.
        }
        return;
      }
    },
    [stopPolling],
  );

  const applyProgressEvent = useCallback(
    (event: JobProgress) => {
      if (isTerminalRef.current && !isTerminalStage(event.stage)) {
        return;
      }

      setProgress(event);

      if (isTerminalStage(event.stage)) {
        const terminalError = event.error_code
          ? { code: event.error_code, message: "" }
          : undefined;

        void handleTerminal(
          event.job_id,
          terminalStatusFromStage(event.stage),
          terminalError,
        );
      }
    },
    [handleTerminal],
  );

  const syncFromStatus = useCallback(
    async (id: string) => {
      if (isTerminalRef.current) {
        return;
      }

      try {
        const response = await getJobStatus(id);
        if (isTerminalRef.current) {
          return;
        }

        setStatus(response.status);
        setProgress(response.progress);
        setBatchItems(response.batch_items ?? []);

        if (!isActiveStatus(response.status)) {
          await handleTerminal(id, response.status, response.error);
          return;
        }

        // Mid-batch: pull finished notes so the UI can preview/switch early.
        const completedCount = (response.batch_items ?? []).filter(
          (item) => item.status === "completed",
        ).length;
        if (completedCount > 0) {
          try {
            const jobResult = await getJobResult(id);
            if (!isTerminalRef.current) {
              setResult(jobResult);
            }
          } catch {
            // Still JOB_NOT_READY or transient; ignore until next poll/terminal.
          }
        }
      } catch {
        // Polling is a fallback; ignore transient failures.
      }
    },
    [handleTerminal],
  );

  const startPolling = useCallback(
    (id: string) => {
      stopPolling();
      pollRef.current = setInterval(() => {
        void syncFromStatus(id);
      }, POLL_INTERVAL_MS);
    },
    [stopPolling, syncFromStatus],
  );

  const claimJob = useCallback(
    (id: string) => {
      jobIdRef.current = id;
      setJobId(id);

      const buffered = pendingEventsRef.current.filter((event) => event.job_id === id);
      pendingEventsRef.current = [];

      for (const event of buffered) {
        applyProgressEvent(event);
      }

      if (!isTerminalRef.current) {
        setIsActive(true);
        setStatus("queued");
        startPolling(id);
        void syncFromStatus(id);
      }
    },
    [applyProgressEvent, startPolling, syncFromStatus],
  );

  const reset = useCallback(() => {
    cleanupListener();
    stopPolling();
    jobIdRef.current = null;
    pendingEventsRef.current = [];
    isTerminalRef.current = false;
    startInFlightRef.current = false;
    setJobId(null);
    setStatus(null);
    setProgress(null);
    setResult(null);
    setError(null);
    setBatchItems([]);
    setIsActive(false);
    setCancelRequested(false);
  }, [cleanupListener, stopPolling]);

  const start = useCallback(
    async (urls: string[]) => {
      if (startInFlightRef.current) {
        return;
      }
      // Block only while a non-terminal job is claimed; allow retry after failure/cancel/done.
      if (jobIdRef.current !== null && !isTerminalRef.current) {
        return;
      }

      startInFlightRef.current = true;
      reset();
      startInFlightRef.current = true;
      setIsActive(true);

      const unlisten = await listenJobProgress((event) => {
        const claimedId = jobIdRef.current;

        if (claimedId) {
          if (event.job_id !== claimedId) {
            return;
          }
          applyProgressEvent(event);
          return;
        }

        pendingEventsRef.current.push(event);
      });

      unlistenRef.current = unlisten;

      try {
        const response = await startNoteJob(urls);
        claimJob(response.job_id);
      } catch (err) {
        cleanupListener();
        stopPolling();
        jobIdRef.current = null;
        pendingEventsRef.current = [];
        isTerminalRef.current = false;
        startInFlightRef.current = false;
        setIsActive(false);
        setJobId(null);
        throw err;
      }
    },
    [reset, applyProgressEvent, claimJob, cleanupListener, stopPolling],
  );

  const cancel = useCallback(async () => {
    const id = jobIdRef.current;
    if (!id) {
      return;
    }

    const response = await cancelJob(id);
    if (!isTerminalRef.current) {
      setStatus(response.status);
    }
    if (response.cancel_requested) {
      setCancelRequested(true);
    }
  }, []);

  useEffect(() => {
    return () => {
      cleanupListener();
      stopPolling();
    };
  }, [cleanupListener, stopPolling]);

  return {
    jobId,
    status,
    progress,
    result,
    error,
    batchItems,
    isActive,
    cancelRequested,
    start,
    cancel,
    reset,
  };
}
