import { Outlet } from "react-router-dom";
import Sidebar from "./Sidebar";
import { ToastContainer } from "./Toast";
import { useDownloadProgress } from "@/hooks/useDownloadProgress";
import { useConfig } from "@/hooks/useConfig";

export default function Layout() {
  // enables global download progress listener (Tauri events)
  useDownloadProgress();
  // syncs settings-store with backend config on mount
  useConfig();

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
