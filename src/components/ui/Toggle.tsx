import { cn } from "@/lib/utils";

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  description?: string;
  disabled?: boolean;
  size?: "sm" | "md";
}

function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  size = "md",
}: ToggleProps) {
  const trackSize = size === "sm" ? "h-[18px] w-[32px]" : "h-[22px] w-10";
  const thumbSize =
    size === "sm" ? "h-[14px] w-[14px]" : "h-[18px] w-[18px]";
  const thumbTranslate = size === "sm" ? "translate-x-[14px]" : "translate-x-[18px]";

  if (!label) {
    return (
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative shrink-0 rounded-full transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
          trackSize,
          checked ? "bg-primary" : "bg-muted",
        )}
      >
        <span
          className={cn(
            "absolute top-[2px] left-[2px] rounded-full bg-white transition-transform duration-150",
            thumbSize,
            checked && thumbTranslate,
          )}
        />
      </button>
    );
  }

  return (
    <div className="flex items-center justify-between rounded-lg px-3 py-3 transition-colors hover:bg-muted/50">
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-foreground">{label}</p>
        {description && (
          <p className="mt-0.5 text-xs text-muted-foreground/60">
            {description}
          </p>
        )}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative shrink-0 rounded-full transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
          trackSize,
          checked ? "bg-primary" : "bg-muted",
        )}
      >
        <span
          className={cn(
            "absolute top-[2px] left-[2px] rounded-full bg-white transition-transform duration-150",
            thumbSize,
            checked && thumbTranslate,
          )}
        />
      </button>
    </div>
  );
}

export { Toggle, type ToggleProps };
