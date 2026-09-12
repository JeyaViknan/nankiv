/**
 * Transient feedback, with an escape hatch.
 *
 * Deletion is offered without a confirmation dialog, which is only defensible
 * because the toast carries Undo. The common case then costs nothing, and the
 * rare mistake is still recoverable — which is the better trade than making
 * everyone confirm every time.
 */

import { useStore } from "../lib/store";

export function Toast() {
  const { toast, dismissToast } = useStore();
  if (!toast) return null;

  return (
    <div className="toast" role="status" aria-live="polite">
      <span>{toast.message}</span>
      {toast.undo && (
        <button
          className="toast-action"
          onClick={() => {
            const run = toast.undo;
            dismissToast();
            run?.();
          }}
        >
          Undo
        </button>
      )}
    </div>
  );
}
