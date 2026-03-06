import { BrowserRouter, Routes, Route } from "react-router-dom";
import { lazy, Suspense, useEffect } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useSettingsStore } from "./stores/settings-store";
import { autoUpdateExtensions } from "./lib/tauri";
import { useLibraryStore, migrateFromLocalStorage } from "./stores/library-store";
import Layout from "./components/ui/Layout";
import { UpdateModal } from "./components/ui/UpdateModal";
import { usePlatform } from "./hooks/usePlatform";
import Home from "./pages/Home";

// lazy-loaded routes – keeps initial bundle small for faster first paint
const Search = lazy(() => import("./pages/Search"));
const ExtensionManga = lazy(() => import("./pages/ExtensionManga"));
const MangaDetail = lazy(() => import("./pages/MangaDetail"));
const Downloads = lazy(() => import("./pages/Downloads"));
const Extensions = lazy(() => import("./pages/Extensions"));
const Settings = lazy(() => import("./pages/Settings"));

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5,
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function App() {
  const { isMobile } = usePlatform();

  // auto-update extension catalog (only if enabled in settings)
  useEffect(() => {
    async function startup() {
      // migrate localStorage data to SQLite
      await migrateFromLocalStorage();

      // load library from SQLite
      await useLibraryStore.getState().loadLibrary();

      const { loadAutoUpdateExtensions } =
        useSettingsStore.getState();

      // load persisted config before deciding
      await loadAutoUpdateExtensions();
      const { autoUpdateExtensions: enabledAfterLoad } =
        useSettingsStore.getState();

      if (!enabledAfterLoad) {
        console.info("[auto-update] disabled in settings");
        return;
      }

      try {
        const result = await autoUpdateExtensions();
        if (result.updated.length > 0) {
          console.info(
            `[auto-update] ${result.updated.length} extension(s) updated:`,
            result.updated.map((e) => `${e.name} v${e.fromVersion}->v${e.toVersion}`),
          );
        }
        if (result.failed.length > 0) {
          console.warn("[auto-update] failures:", result.failed);
        }
      } catch (err) {
        console.warn("[auto-update]", err);
      }
    }

    startup();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        {!isMobile && <UpdateModal />}
        <Suspense>
          <Routes>
            <Route element={<Layout />}>
              <Route path="/" element={<Home />} />
              <Route path="/search" element={<Search />} />
              <Route path="/library/:source" element={<ExtensionManga />} />
              <Route path="/manga/:id" element={<MangaDetail />} />
              <Route path="/downloads" element={<Downloads />} />
              <Route path="/extensions" element={<Extensions />} />
              <Route path="/settings" element={<Settings />} />
            </Route>
          </Routes>
        </Suspense>
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;
