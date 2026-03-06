import { NavLink } from "react-router-dom";
import { Download, Library, Plus, Puzzle, Settings } from "lucide-react";
import { useTranslation } from "@/hooks/useTranslation";

const items = [
  { to: "/", icon: Library, labelKey: "sidebar.library" as const },
  { to: "/search", icon: Plus, labelKey: "sidebar.addWork" as const },
  { to: "/downloads", icon: Download, labelKey: "sidebar.downloads" as const },
  { to: "/extensions", icon: Puzzle, labelKey: "sidebar.extensions" as const },
  { to: "/settings", icon: Settings, labelKey: "sidebar.settings" as const },
];

export default function BottomNav() {
  const { t } = useTranslation();

  return (
    <nav className="fixed inset-x-0 bottom-0 z-40 border-t border-[hsl(var(--sidebar-border))] bg-[hsl(var(--card))/0.96] px-2 pb-[max(0.5rem,env(safe-area-inset-bottom))] pt-2 backdrop-blur supports-[backdrop-filter]:bg-[hsl(var(--card))/0.82]">
      <ul className="mx-auto grid max-w-xl grid-cols-5 gap-1">
        {items.map(({ to, icon: Icon, labelKey }) => (
          <li key={to}>
            <NavLink
              to={to}
              end={to === "/"}
              className={({ isActive }) =>
                `flex min-h-12 flex-col items-center justify-center gap-1 rounded-lg px-1 py-1 text-[10px] font-medium no-underline transition-colors ${
                  isActive
                    ? "bg-[hsl(var(--primary)/0.14)] text-[hsl(var(--primary))]"
                    : "text-[hsl(var(--muted-foreground))] hover:bg-[hsl(var(--muted))]"
                }`
              }
            >
              <Icon size={17} strokeWidth={2} />
              <span className="truncate">{t(labelKey)}</span>
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  );
}
