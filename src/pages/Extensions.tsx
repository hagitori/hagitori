import { useState } from "react";
import {
  Search,
  Globe,
  RefreshCw,
  Plus,
  ArrowUpCircle,
} from "lucide-react";
import { useTranslation } from "@/hooks/useTranslation";
import { useExtensions } from "@/hooks/useExtensions";
import { useSync } from "@/hooks/useSync";
import { useLibraryStore } from "@/stores/library-store";
import { setExtensionLang, autoUpdateExtensions } from "@/lib/tauri";
import { Button, Input, Badge, useToast } from "@/components/ui";
import { useExtensionFilters } from "@/hooks/useExtensionFilters";
import { InstalledTab } from "@/components/extensions/InstalledTab";
import { CatalogTab } from "@/components/extensions/CatalogTab";
import { AddRepoModal } from "@/components/extensions/AddRepoModal";
import type { ExtensionUpdateInfo } from "@/types";

type Tab = "installed" | "catalog";

export default function Extensions() {
  const { t } = useTranslation();
  const {
    extensions,
    isLoading: loading,
    remove,
  } = useExtensions();

  const {
    updates,
    isCatalogLoading,
    isUpdatesLoading,
    install: installFromCatalog,
    update: updateFromCatalog,
    isInstalling,
    isUpdating,
    installingId,
    updatingId,
    refetch,
  } = useSync();

  const [search, setSearch] = useState("");
  const [activeTab, setActiveTab] = useState<Tab>("installed");
  const [showRepoModal, setShowRepoModal] = useState(false);
  const [isUpdatingAll, setIsUpdatingAll] = useState(false);
  const toast = useToast();

  const extensionLangs = useLibraryStore((s) => s.extensionLangs);
  const storeSetExtensionLang = useLibraryStore((s) => s.setExtensionLang);

  const {
    filteredExtensions,
    filteredUpdates,
    availableLangs,
    selectedLangs,
    toggleLang,
    clearLangs,
    updatesAvailableCount,
  } = useExtensionFilters({ extensions, updates, search });

  const handleLangChange = async (extensionId: string, lang: string) => {
    try {
      await setExtensionLang(extensionId, lang);
      await storeSetExtensionLang(extensionId, lang);
    } catch (err) {
      toast.error(String(err));
    }
  };

  const handleRemove = async (id: string) => {
    try {
      await remove(id);
      toast.success(t("extensions.removeSuccess"));
    } catch (err) {
      toast.error(t("extensions.removeError"));
    }
  };

  const handleInstall = async (ext: ExtensionUpdateInfo) => {
    try {
      await installFromCatalog(ext.id);
      toast.success(t("extensions.installSuccess").replace("{name}", ext.name));
    } catch (err) {
      toast.error(t("extensions.installError"));
    }
  };

  const handleUpdate = async (ext: ExtensionUpdateInfo) => {
    try {
      await updateFromCatalog(ext.id);
      toast.success(t("extensions.updateSuccess").replace("{name}", ext.name));
    } catch (err) {
      toast.error(t("extensions.updateError"));
    }
  };

  const handleUpdateAll = async () => {
    setIsUpdatingAll(true);
    try {
      const result = await autoUpdateExtensions();
      if (result.updated.length > 0) {
        toast.success(
          `${result.updated.length} ${t("extensions.updateAvailable").toLowerCase()} ✓`,
        );
      }
      if (result.failed.length > 0) {
        toast.error(
          `${result.failed.length} ${t("extensions.updateError").toLowerCase()}`,
        );
      }
      refetch();
    } catch (err) {
      toast.error(t("extensions.updateError"));
    } finally {
      setIsUpdatingAll(false);
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-foreground">
            {t("extensions.title")}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {extensions.length} {t("extensions.installed").toLowerCase()}
            {updatesAvailableCount > 0 && (
              <span className="ml-2 text-[hsl(var(--warning))]">
                · {updatesAvailableCount} {t("extensions.updateAvailable").toLowerCase()}
              </span>
            )}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {updatesAvailableCount > 0 && (
            <Button
              variant="primary"
              size="sm"
              icon={<ArrowUpCircle size={14} strokeWidth={2} />}
              onClick={handleUpdateAll}
              disabled={isUpdatingAll}
            >
              {isUpdatingAll ? t("extensions.updating") : t("extensions.updateAll")}
            </Button>
          )}
          <Button
            variant="outline"
            size="sm"
            icon={<Plus size={14} strokeWidth={2} />}
            onClick={() => setShowRepoModal(true)}
          >
            {t("extensions.addRepo")}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            icon={<RefreshCw size={14} strokeWidth={2} />}
            onClick={refetch}
            disabled={isCatalogLoading || isUpdatesLoading}
          >
            {t("extensions.checkUpdates")}
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <div className="mb-4 flex items-center gap-4 border-b border-border">
        <button
          className={`pb-2 text-sm font-medium transition-colors ${
            activeTab === "installed"
              ? "border-b-2 border-primary text-foreground"
              : "text-muted-foreground hover:text-foreground"
          }`}
          onClick={() => setActiveTab("installed")}
        >
          {t("extensions.tabInstalled")} ({extensions.length})
        </button>
        <button
          className={`pb-2 text-sm font-medium transition-colors ${
            activeTab === "catalog"
              ? "border-b-2 border-primary text-foreground"
              : "text-muted-foreground hover:text-foreground"
          }`}
          onClick={() => setActiveTab("catalog")}
        >
          {t("extensions.tabCatalog")}
          {updatesAvailableCount > 0 && (
            <Badge variant="warning" size="sm" className="ml-1.5">
              {updatesAvailableCount}
            </Badge>
          )}
        </button>
      </div>

      {/* Search */}
      <div className="mb-4">
        <Input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={`${t("extensions.title")}...`}
          icon={<Search size={14} strokeWidth={2} />}
          className="h-9"
        />
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === "installed" && (
          <InstalledTab
            extensions={filteredExtensions}
            loading={loading}
            extensionLangs={extensionLangs}
            onLangChange={handleLangChange}
            onRemove={handleRemove}
          />
        )}

        {activeTab === "catalog" && (
          <>
            {/* language filter */}
            {availableLangs.length > 1 && (
              <div className="mb-3 flex flex-wrap items-center gap-2">
                <Globe size={14} className="shrink-0 text-muted-foreground/60" />
                <span className="text-xs text-muted-foreground/60">
                  {t("extensions.filterLanguage")}:
                </span>
                {availableLangs.map((lang) => (
                  <button
                    key={lang}
                    onClick={() => toggleLang(lang)}
                    className={`rounded-md px-2 py-0.5 text-xs font-medium transition-colors ${
                      selectedLangs.has(lang)
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted text-muted-foreground hover:bg-muted/80"
                    }`}
                  >
                    {lang}
                  </button>
                ))}
                {selectedLangs.size > 0 && (
                  <button
                    onClick={() => clearLangs()}
                    className="text-[10px] text-muted-foreground/50 hover:text-foreground"
                  >
                    {t("extensions.allLanguages")}
                  </button>
                )}
              </div>
            )}
            <CatalogTab
              updates={filteredUpdates}
              loading={isCatalogLoading || isUpdatesLoading}
              onInstall={handleInstall}
              onUpdate={handleUpdate}
              isInstalling={isInstalling}
              isUpdating={isUpdating}
              installingId={installingId}
              updatingId={updatingId}
            />
          </>
        )}
      </div>

      {/* modal: add repository */}
      {showRepoModal && (
        <AddRepoModal
          onClose={() => setShowRepoModal(false)}
          onSuccess={() => {
            setShowRepoModal(false);
            refetch();
            setActiveTab("catalog");
          }}
        />
      )}
    </div>
  );
}


