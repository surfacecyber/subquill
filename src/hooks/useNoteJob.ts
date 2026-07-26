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
  const [isActive, setIsActive] = useState(false);
  const [cancelRequested, setCancelRequested] = useState(false);

  const unlistenRef = useRef<UnlistenFn | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const jobIdRef = useRef<string | null>(null);
  const pendingEventsRef = useRef<JobProgress[]>([]);
  const isTerminalRef = useRef(false);

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
      stopPolling();
      setIsActive(false);
      setStatus(terminalStatus);

      if (terminalStatus === "completed") {
        try {
          const jobResult = await getJobResult(id);
          setResult(jobResult);
          setError(null);
        } catch (err) {
          setError(err as AppErrorPayload);
        }
        return;
      }

      if (terminalError) {
        setError(terminalError);
      }
      setResult(null);
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
          ? { code: event.error_code, message: event.error_code }
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

        if (!isActiveStatus(response.status)) {
          await handleTerminal(id, response.status, response.error);
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
    setJobId(null);
    setStatus(null);
    setProgress(null);
    setResult(null);
    setError(null);
    setIsActive(false);
    setCancelRequested(false);
  }, [cleanupListener, stopPolling]);

  const start = useCallback(
    async (url: string) => {
      reset();

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
        const response = await startNoteJob(url);
        claimJob(response.job_id);
      } catch (err) {
        cleanupListener();
        stopPolling();
        jobIdRef.current = null;
        pendingEventsRef.current = [];
        isTerminalRef.current = false;
        setIsActive(false);
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
    isActive,
    cancelRequested,
    start,
    cancel,
    reset,
  };
}
