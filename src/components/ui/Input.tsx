import {
  forwardRef,
  type InputHTMLAttributes,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  description?: string;
  error?: string;
  icon?: ReactNode;
  iconRight?: ReactNode;
}

const Input = forwardRef<HTMLInputElement, InputProps>(
  (
    { label, description, error, icon, iconRight, className, id, ...props },
    ref,
  ) => {
    const inputId = id || label?.toLowerCase().replace(/\s+/g, "-");

    return (
      <div className="flex flex-col gap-1.5">
        {label && (
          <label
            htmlFor={inputId}
            className="text-sm font-medium text-foreground"
          >
            {label}
          </label>
        )}
        {description && (
          <p className="text-xs text-muted-foreground/60">{description}</p>
        )}
        <div className="relative">
          {icon && (
            <span className="absolute left-3.5 top-1/2 -translate-y-1/2 text-muted-foreground">
              {icon}
            </span>
          )}
          <input
            ref={ref}
            id={inputId}
            className={cn(
              "h-10 w-full rounded-lg border border-border bg-muted px-3.5 text-sm text-foreground placeholder:text-muted-foreground/60 transition-colors duration-150 focus:border-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-50",
              icon ? "pl-10" : "",
              iconRight ? "pr-10" : "",
              error ? "border-[hsl(var(--danger))] focus:border-[hsl(var(--danger))]" : "",
              className,
            )}
            {...props}
          />
          {iconRight && (
            <span className="absolute right-3.5 top-1/2 -translate-y-1/2 text-muted-foreground">
              {iconRight}
            </span>
          )}
        </div>
        {error && (
          <p className="text-xs text-[hsl(var(--danger))]">{error}</p>
        )}
      </div>
    );
  },
);

Input.displayName = "Input";

export { Input, type InputProps };
