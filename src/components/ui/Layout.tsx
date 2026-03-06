import { Outlet } from "react-router-dom";
import Sidebar from "./Sidebar";
import BottomNav from "./BottomNav";
import { ToastContainer } from "./Toast";
import { useDownloadProgress } from "@/hooks/useDownloadProgress";
import { useConfig } from "@/hooks/useConfig";
import { usePlatform } from "@/hooks/usePlatform";

export default function Layout() {
  // enables global download progress listener (Tauri events)
  useDownloadProgress();
  // syncs settings-store with backend config on mount
  useConfig();

  const { isMobile } = usePlatform();

  if (isMobile) {
    return (
      <div className="flex h-screen w-screen overflow-hidden bg-[hsl(var(--background))]">
        <main className="flex-1 overflow-y-auto px-4 pb-24 pt-[max(3rem,env(safe-area-inset-top))] text-[hsl(var(--foreground))]">
          <Outlet />
        </main>
        <BottomNav />
        <ToastContainer />
      </div>
    );
  }

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[hsl(var(--background))]">
      <Sidebar />
      <main className="flex-1 overflow-y-auto p-6 text-[hsl(var(--foreground))]">
        <Outlet />
      </main>
      <ToastContainer />
    </div>
  );
}
