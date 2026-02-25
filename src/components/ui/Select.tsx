import { forwardRef, useState, useRef, useEffect, useCallback, type ReactNode } from "react";
import { cn } from "@/lib/utils";

interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps {
  label?: string;
  description?: string;
  options: readonly (string | SelectOption)[];
  error?: string;
  icon?: ReactNode;
  value?: string;
  onChange?: (value: string) => void;
  className?: string;
  id?: string;
  disabled?: boolean;
}

const Select = forwardRef<HTMLDivElement, SelectProps>(
  (
    {
      label,
      description,
      options,
      error,
      icon,
      value,
      onChange,
      className,
      id,
      disabled,
    },
    ref,
  ) => {
    const [open, setOpen] = useState(false);
    const [highlightedIndex, setHighlightedIndex] = useState(-1);
    const containerRef = useRef<HTMLDivElement>(null);
    const listboxRef = useRef<HTMLDivElement>(null);
    const selectId = id || label?.toLowerCase().replace(/\s+/g, "-");
    const listboxId = `${selectId}-listbox`;

    const normalizedOptions: SelectOption[] = options.map((opt) =>
      typeof opt === "string" ? { value: opt, label: opt.toUpperCase() } : opt,
    );

    const selectedLabel =
      normalizedOptions.find((opt) => opt.value === value)?.label ??
      value?.toUpperCase() ??
      "";

    // close on outside click
    useEffect(() => {
      if (!open) return;
      const handler = (e: MouseEvent) => {
        if (
          containerRef.current &&
          !containerRef.current.contains(e.target as Node)
        ) {
          setOpen(false);
        }
      };
      document.addEventListener("mousedown", handler);
      return () => document.removeEventListener("mousedown", handler);
    }, [open]);

    // initialize highlight to selected option when opening
    useEffect(() => {
      if (open) {
        const idx = normalizedOptions.findIndex((opt) => opt.value === value);
        setHighlightedIndex(idx >= 0 ? idx : 0);
      }
    }, [open]); // eslint-disable-line react-hooks/exhaustive-deps, only reset on open/close toggle

    // scroll highlighted option into view
    useEffect(() => {
      if (!open || highlightedIndex < 0) return;
      const optionEl = listboxRef.current?.children[highlightedIndex] as HTMLElement | undefined;
      optionEl?.scrollIntoView({ block: "nearest" });
    }, [open, highlightedIndex]);

    const selectOption = useCallback(
      (opt: SelectOption) => {
        onChange?.(opt.value);
        setOpen(false);
      },
      [onChange],
    );

    // keyboard handler for both trigger and listbox
    const handleKeyDown = useCallback(
      (e: React.KeyboardEvent) => {
        switch (e.key) {
          case "Escape":
            setOpen(false);
            break;
          case "Enter":
          case " ":
            e.preventDefault();
            if (open && highlightedIndex >= 0) {
              selectOption(normalizedOptions[highlightedIndex]);
            } else {
              setOpen(true);
            }
            break;
          case "ArrowDown":
            e.preventDefault();
            if (!open) {
              setOpen(true);
            } else {
              setHighlightedIndex((i) =>
                i < normalizedOptions.length - 1 ? i + 1 : 0,
              );
            }
            break;
          case "ArrowUp":
            e.preventDefault();
            if (!open) {
              setOpen(true);
            } else {
              setHighlightedIndex((i) =>
                i > 0 ? i - 1 : normalizedOptions.length - 1,
              );
            }
            break;
          case "Home":
            if (open) {
              e.preventDefault();
              setHighlightedIndex(0);
            }
            break;
          case "End":
            if (open) {
              e.preventDefault();
              setHighlightedIndex(normalizedOptions.length - 1);
            }
            break;
        }
      },
      [open, highlightedIndex, normalizedOptions, selectOption],
    );

    const activeDescendantId =
      open && highlightedIndex >= 0
        ? `${selectId}-option-${highlightedIndex}`
        : undefined;

    return (
      <div className="flex flex-col gap-1.5" ref={ref}>
        {label && (
          <label
            htmlFor={selectId}
            className="text-sm font-medium text-foreground"
          >
            {label}
          </label>
        )}
        {description && (
          <p className="text-xs text-muted-foreground/60">{description}</p>
        )}
        <div className="relative" ref={containerRef}>
          <button
            id={selectId}
            type="button"
            role="combobox"
            aria-haspopup="listbox"
            aria-expanded={open}
            aria-controls={open ? listboxId : undefined}
            aria-activedescendant={activeDescendantId}
            disabled={disabled}
            onClick={() => setOpen((v) => !v)}
            onKeyDown={handleKeyDown}
            className={cn(
              "flex h-9 w-full items-center justify-between rounded-lg border border-border bg-muted px-3 text-xs font-medium text-foreground transition-colors duration-150 hover:border-primary/50 focus:border-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-50",
              icon ? "pl-9" : "",
              error ? "border-[hsl(var(--danger))]" : "",
              className,
            )}
          >
            {icon && (
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground">
                {icon}
              </span>
            )}
            <span className="truncate whitespace-nowrap">{selectedLabel}</span>
            {/* Chevron */}
            <svg
              className={cn(
                "ml-2 h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform duration-150",
                open && "rotate-180",
              )}
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M4 6l4 4 4-4" />
            </svg>
          </button>

          {open && (
            <div
              ref={listboxRef}
              id={listboxId}
              role="listbox"
              aria-label={label}
              className="absolute right-0 top-full z-50 mt-1 max-h-48 min-w-full overflow-auto rounded-lg border border-border bg-popover py-1 shadow-lg shadow-black/20"
            >
              {normalizedOptions.map((opt, idx) => (
                <button
                  key={opt.value}
                  id={`${selectId}-option-${idx}`}
                  type="button"
                  role="option"
                  aria-selected={opt.value === value}
                  onClick={() => selectOption(opt)}
                  onMouseEnter={() => setHighlightedIndex(idx)}
                  className={cn(
                    "flex w-full items-center justify-between gap-2 px-3 py-1.5 text-xs whitespace-nowrap transition-colors",
                    opt.value === value
                      ? "bg-primary/10 font-semibold text-primary"
                      : "text-popover-foreground",
                    idx === highlightedIndex && "bg-muted",
                  )}
                >
                  <span>{opt.label}</span>
                  {opt.value === value ? (
                    <svg
                      className="h-3 w-3 shrink-0"
                      viewBox="0 0 16 16"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2.5"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      <path d="M3 8.5l3.5 3.5 6.5-8" />
                    </svg>
                  ) : (
                    <span className="w-3 shrink-0" />
                  )}
                </button>
              ))}
            </div>
          )}
        </div>
        {error && (
          <p className="text-xs text-[hsl(var(--danger))]">{error}</p>
        )}
      </div>
    );
  },
);

Select.displayName = "Select";

export { Select, type SelectOption, type SelectProps };
