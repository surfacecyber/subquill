import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

export type NotificationType = "success" | "error" | "info";

type NotificationItem = {
  id: number;
  type: NotificationType;
  message: string;
};

type NotificationApi = {
  notify: (type: NotificationType, message: string) => void;
  clear: () => void;
};

const NotificationContext = createContext<NotificationApi | null>(null);

const AUTO_DISMISS_MS = 4500;
const MAX_VISIBLE = 4;

interface NotificationProviderProps {
  children: ReactNode;
  dismissLabel?: string;
}

export function NotificationProvider({
  children,
  dismissLabel = "Dismiss",
}: NotificationProviderProps) {
  const [items, setItems] = useState<NotificationItem[]>([]);
  const nextIdRef = useRef(1);
  const timersRef = useRef(new Map<number, number>());

  const dismiss = useCallback((id: number) => {
    const timer = timersRef.current.get(id);
    if (timer !== undefined) {
      window.clearTimeout(timer);
      timersRef.current.delete(id);
    }
    setItems((prev) => prev.filter((item) => item.id !== id));
  }, []);

  const clear = useCallback(() => {
    for (const timer of timersRef.current.values()) {
      window.clearTimeout(timer);
    }
    timersRef.current.clear();
    setItems([]);
  }, []);

  const notify = useCallback(
    (type: NotificationType, message: string) => {
      const id = nextIdRef.current++;
      setItems((prev) => {
        const next = [...prev, { id, type, message }];
        return next.length > MAX_VISIBLE ? next.slice(-MAX_VISIBLE) : next;
      });
      const timer = window.setTimeout(() => dismiss(id), AUTO_DISMISS_MS);
      timersRef.current.set(id, timer);
    },
    [dismiss],
  );

  useEffect(() => {
    return () => {
      for (const timer of timersRef.current.values()) {
        window.clearTimeout(timer);
      }
    };
  }, []);

  const value = useMemo(() => ({ notify, clear }), [notify, clear]);

  return (
    <NotificationContext.Provider value={value}>
      {children}
      {createPortal(
        <div
          className="notification-stack"
          aria-live="polite"
          aria-relevant="additions"
        >
          {items.map((item) => (
            <div
              key={item.id}
              className={`notification notification-${item.type}`}
              role={item.type === "error" ? "alert" : "status"}
            >
              <p className="notification-message">{item.message}</p>
              <button
                type="button"
                className="notification-close"
                aria-label={dismissLabel}
                onClick={() => dismiss(item.id)}
              >
                ×
              </button>
            </div>
          ))}
        </div>,
        document.body,
      )}
    </NotificationContext.Provider>
  );
}

export function useNotify(): NotificationApi {
  const ctx = useContext(NotificationContext);
  if (!ctx) {
    throw new Error("useNotify must be used within NotificationProvider");
  }
  return ctx;
}
