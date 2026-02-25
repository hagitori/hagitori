import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listExtensions,
  removeCatalogExtension,
} from "../lib/tauri";

/**
 * TanStack Query wrapper for extensions.
 * - `extensions` list (cached)
 * - `remove(id)` mutation with invalidation removes from disk, database and memory
 */
export function useExtensions() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ["extensions"],
    queryFn: async () => {
      const exts = await listExtensions();
      return exts;
    },
  });

  const removeMutation = useMutation({
    mutationFn: (id: string) => removeCatalogExtension(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["extensions"] });
    },
  });

  return {
    extensions: query.data ?? [],
    isLoading: query.isLoading,
    error: query.error,
    remove: removeMutation.mutateAsync,
    isRemoving: removeMutation.isPending,
  };
}
