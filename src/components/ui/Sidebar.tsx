import { useState, useEffect } from "react";
import { NavLink } from "react-router-dom";
import { Library, Download, Puzzle, Settings, Plus } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { useTranslation } from "@/hooks/useTranslation";
import bannerImg from "@/assets/banner.png";

const navItems = [
  { to: "/", icon: Library, labelKey: "sidebar.library" as const },
  { to: "/downloads", icon: Download, labelKey: "sidebar.downloads" as const },
  {
    to: "/extensions",
    icon: Puzzle,
    labelKey: "sidebar.extensions" as const,
  },
  {
    to: "/settings",
    icon: Settings,
    labelKey: "sidebar.settings" as const,
  },
];

export default function Sidebar() {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then((v) => setVersion(v)).catch(() => setVersion("?"));
  }, []);

  return (
    <aside className="flex h-full w-[220px] shrink-0 flex-col border-r border-[hsl(var(--sidebar-border))] bg-[hsl(var(--card))]">
      {/* Logo */}
      <div className="px-4 pt-4 pb-4">
        <img
          src={bannerImg}
          alt="Hagitori"
          className="w-full max-w-[180px] h-auto select-none"
          draggable={false}
        />
      </div>

      {/* Downaload button */}
      <div className="px-3 pb-2">
        <NavLink
          to="/search"
          className={({ isActive }) =>
            `flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors duration-150 no-underline ${
              isActive
                ? "bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))]"
                : "bg-[hsl(var(--primary)/0.1)] text-[hsl(var(--primary))] hover:bg-[hsl(var(--primary)/0.2)]"
            }`
          }
        >
          <Plus size={18} strokeWidth={2} className="shrink-0" />
          <span className="leading-none">{t("sidebar.addWork")}</span>
        </NavLink>
      </div>

      {/* Navigation */}
      <nav className="flex flex-1 flex-col gap-1 px-3 pt-3">
        {navItems.map(({ to, icon: Icon, labelKey }) => (
          <NavLink
            key={to}
            to={to}
            end={to === "/"}
            className={({ isActive }) =>
              `flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors duration-150 no-underline ${
                isActive
                  ? "bg-[hsl(var(--primary)/0.1)] text-[hsl(var(--primary))]"
                  : "text-[hsl(var(--muted-foreground))] hover:bg-[hsl(var(--muted))] hover:text-[hsl(var(--foreground))]"
              }`
            }
          >
            <Icon size={18} strokeWidth={2} />
            <span>{t(labelKey)}</span>
          </NavLink>
        ))}
      </nav>

      {/* Version footer */}
      <div className="px-5 py-4">
        <p className="text-[11px] font-medium tracking-wider text-[hsl(var(--muted-foreground)/0.6)]">
          v{version}
        </p>
      </div>
    </aside>
  );
}
