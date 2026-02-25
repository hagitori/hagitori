import { CheckCircle, AlertTriangle, Info, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useToastStore, type ToastVariant } from "@/stores/toast-store";

const variantConfig: Record<
  ToastVariant,
  { icon: typeof Info; colorClass: string; bgClass: string }
> = {
  success: {
    icon: CheckCircle,
    colorClass: "text-[hsl(var(--success))]",
    bgClass: "border-[hsl(var(--success))]/20 bg-[hsl(var(--success))]/5",
  },
  error: {
    icon: AlertTriangle,
    colorClass: "text-[hsl(var(--danger))]",
    bgClass: "border-[hsl(var(--danger))]/20 bg-[hsl(var(--danger))]/5",
  },
  warning: {
    icon: AlertTriangle,
    colorClass: "text-[hsl(var(--warning))]",
    bgClass: "border-[hsl(var(--warning))]/20 bg-[hsl(var(--warning))]/5",
  },
  info: {
    icon: Info,
    colorClass: "text-[hsl(var(--info))]",
    bgClass: "border-[hsl(var(--info))]/20 bg-[hsl(var(--info))]/5",
  },
};

function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);
  const removeToast = useToastStore((s) => s.removeToast);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((toast) => {
        const config = variantConfig[toast.variant];
        const Icon = config.icon;

        return (
          <div
            key={toast.id}
            className={cn(
              "flex items-center gap-3 rounded-lg border px-4 py-3 shadow-lg animate-in slide-in-from-right-full duration-200",
              config.bgClass,
            )}
            style={{ minWidth: 280, maxWidth: 400 }}
          >
            <Icon size={16} strokeWidth={2} className={config.colorClass} />
            <p className="flex-1 text-sm text-foreground">{toast.message}</p>
            <button
              type="button"
              onClick={() => removeToast(toast.id)}
              className="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
            >
              <X size={14} strokeWidth={2} />
            </button>
          </div>
        );
      })}
    </div>
  );
}

/** convenience hook for using toasts */
function useToast() {
  const addToast = useToastStore((s) => s.addToast);

  return {
    success: (message: string) => addToast(message, "success"),
    error: (message: string) => addToast(message, "error"),
    warning: (message: string) => addToast(message, "warning"),
    info: (message: string) => addToast(message, "info"),
    toast: addToast,
  };
}

export { ToastContainer, useToast };
