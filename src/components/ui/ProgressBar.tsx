import { cn } from "@/lib/utils";

type ProgressBarVariant = "primary" | "success" | "warning" | "danger";

interface ProgressBarProps {
  value: number;
  max?: number;
  variant?: ProgressBarVariant;
  showLabel?: boolean;
  label?: string;
  size?: "sm" | "md";
  className?: string;
}

const variantStyles: Record<ProgressBarVariant, string> = {
  primary: "bg-primary",
  success: "bg-[hsl(var(--success))]",
  warning: "bg-[hsl(var(--warning))]",
  danger: "bg-[hsl(var(--danger))]",
};

const sizeStyles = {
  sm: "h-1",
  md: "h-1.5",
};

function ProgressBar({
  value,
  max = 100,
  variant = "primary",
  showLabel = false,
  label,
  size = "md",
  className,
}: ProgressBarProps) {
  const percentage = Math.min(Math.round((value / max) * 100), 100);

  return (
    <div className={cn("flex items-center gap-3", className)}>
      <div
        className={cn(
          "flex-1 overflow-hidden rounded-full bg-muted",
          sizeStyles[size],
        )}
        role="progressbar"
        aria-valuenow={value}
        aria-valuemin={0}
        aria-valuemax={max}
      >
        <div
          className={cn(
            "h-full rounded-full transition-all duration-500 ease-out",
            variantStyles[variant],
          )}
          style={{ width: `${percentage}%` }}
        />
      </div>
      {showLabel && (
        <span className="shrink-0 text-[11px] font-medium tabular-nums text-muted-foreground">
          {label || `${percentage}%`}
        </span>
      )}
    </div>
  );
}

export { ProgressBar, type ProgressBarVariant, type ProgressBarProps };
