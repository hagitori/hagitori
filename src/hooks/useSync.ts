import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  fetchCatalog,
  checkExtensionUpdates,
  installCatalogExtension,
  updateCatalogExtension,
  removeCatalogExtension,
  setExtensionAutoUpdate,
} from "../lib/tauri";
import type { ExtensionCatalog } from "../types";

/**
 * hook for syncing extensions with the GitHub catalog.
 *
 * provides:
 * - `catalog`: remote catalog (cached for 5 min)
 * - `updates`: status of each extension (up-to-date, update_available, etc.)
 * - `install(id)`: installs extension from catalog
 * - `update(id)`: updates extension to latest version
 * - `remove(id)`: removes extension from catalog
 * - `toggleAutoUpdate(id, enabled)`: toggles auto-update
 * - `refetch()`: reloads catalog and updates
 */
export function useSync() {
  const queryClient = useQueryClient();

  // -----------------------------------------------------------------------
  // queries
  // -----------------------------------------------------------------------

  const catalogQuery = useQuery({
    queryKey: ["catalog"],
    queryFn: fetchCatalog,
    staleTime: 5 * 60 * 1000, // 5 minutes
    refetchOnWindowFocus: false,
  });

  const updatesQuery = useQuery({
    queryKey: ["extension-updates"],
    queryFn: () => checkExtensionUpdates(catalogQuery.data!),
    enabled: !!catalogQuery.data,
    staleTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  // -----------------------------------------------------------------------
  // mutations
  // -----------------------------------------------------------------------

  const invalidateAll = () => {
    queryClient.invalidateQueries({ queryKey: ["catalog"] });
    queryClient.invalidateQueries({ queryKey: ["extension-updates"] });
    queryClient.invalidateQueries({ queryKey: ["extensions"] });
  };

  const installMutation = useMutation({
    mutationFn: async (extensionId: string) => {
      const catalog = queryClient.getQueryData<ExtensionCatalog>(["catalog"]) ?? (await fetchCatalog());
      return installCatalogExtension(extensionId, catalog);
    },
    onSuccess: invalidateAll,
  });

  const updateMutation = useMutation({
    mutationFn: async (extensionId: string) => {
      const catalog = queryClient.getQueryData<ExtensionCatalog>(["catalog"]) ?? (await fetchCatalog());
      return updateCatalogExtension(extensionId, catalog);
    },
    onSuccess: invalidateAll,
  });

  const removeMutation = useMutation({
    mutationFn: removeCatalogExtension,
    onSuccess: invalidateAll,
  });

  const autoUpdateMutation = useMutation({
    mutationFn: ({
      extensionId,
      enabled,
    }: {
      extensionId: string;
      enabled: boolean;
    }) => setExtensionAutoUpdate(extensionId, enabled),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["extension-updates"] });
    },
  });

  // -----------------------------------------------------------------------
  // public API
  // -----------------------------------------------------------------------

  return {
    // data
    catalog: catalogQuery.data ?? null,
    updates: updatesQuery.data ?? [],
    isCatalogLoading: catalogQuery.isLoading,
    isUpdatesLoading: updatesQuery.isLoading,
    catalogError: catalogQuery.error,
    updatesError: updatesQuery.error,

    // actions
    install: installMutation.mutateAsync,
    update: updateMutation.mutateAsync,
    remove: removeMutation.mutateAsync,
    toggleAutoUpdate: autoUpdateMutation.mutateAsync,

    // states
    isInstalling: installMutation.isPending,
    isUpdating: updateMutation.isPending,
    isRemoving: removeMutation.isPending,

    // extension ID being processed (for loading UI)
    installingId: installMutation.variables,
    updatingId: updateMutation.variables,
    removingId: removeMutation.variables,

    // refetch
    refetch: () => {
      catalogQuery.refetch();
      updatesQuery.refetch();
    },
  };
}
