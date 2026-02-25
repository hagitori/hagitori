import {
  Puzzle,
  ExternalLink,
  Download,
  ArrowUpCircle,
  CheckCircle2,
} from "lucide-react";
import { Button, Card, Badge, EmptyState, SkeletonCard } from "@/components/ui";
import { useTranslation } from "@/hooks/useTranslation";
import type { ExtensionUpdateInfo } from "@/types";

/** converts numeric versionId to semver format (matches backend logic). */
function formatVersionId(versionId: number): string {
  return `0.1.${Math.max(0, versionId - 1)}`;
}

function StatusBadge({ status }: { status: string }) {
  const { t } = useTranslation();
  switch (status) {
    case "up_to_date":
      return (
        <Badge variant="success" size="sm" className="flex items-center gap-1">
          <CheckCircle2 size={10} />
          {t("extensions.upToDate")}
        </Badge>
      );
    case "update_available":
      return (
        <Badge variant="warning" size="sm" className="flex items-center gap-1">
          <ArrowUpCircle size={10} />
          {t("extensions.updateAvailable")}
        </Badge>
      );
    case "local_newer":
      return (
        <Badge variant="info" size="sm" className="flex items-center gap-1">
          <ArrowUpCircle size={10} />
          {t("extensions.localNewer")}
        </Badge>
      );
    case "orphaned":
      return (
        <Badge variant="danger" size="sm" className="flex items-center gap-1">
          {t("extensions.orphaned")}
        </Badge>
      );
    default:
      return (
        <Badge variant="outline" size="sm">
          {t("extensions.notInstalled")}
        </Badge>
      );
  }
}

export interface CatalogTabProps {
  updates: ExtensionUpdateInfo[];
  loading: boolean;
  onInstall: (ext: ExtensionUpdateInfo) => void;
  onUpdate: (ext: ExtensionUpdateInfo) => void;
  isInstalling: boolean;
  isUpdating: boolean;
  installingId?: string;
  updatingId?: string;
}

export function CatalogTab({
  updates,
  loading,
  onInstall,
  onUpdate,
  isInstalling,
  isUpdating,
  installingId,
  updatingId,
}: CatalogTabProps) {
  const { t } = useTranslation();
  if (loading) {
    return (
      <div className="flex flex-col gap-2">
        {[1, 2, 3, 4].map((i) => (
          <SkeletonCard key={i} />
        ))}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {updates.map((ext) => {
        const isThisInstalling = isInstalling && installingId === ext.id;
        const isThisUpdating = isUpdating && updatingId === ext.id;
        const isBusy = isThisInstalling || isThisUpdating;

        return (
          <Card key={ext.id}>
            <div className="flex items-center gap-4">
              {/* Icon */}
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 overflow-hidden">
                {ext.iconUrl ? (
                  <img
                    src={ext.iconUrl}
                    alt={ext.name}
                    className="h-10 w-10 rounded-lg object-cover"
                  />
                ) : (
                  <Puzzle size={18} strokeWidth={1.5} className="text-primary" />
                )}
              </div>

              {/* Info */}
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <h3 className="text-sm font-semibold text-foreground">{ext.name}</h3>
                  <Badge variant="outline" size="sm">v{formatVersionId(ext.remoteVersionId)}</Badge>
                  <Badge variant="default" size="sm">{ext.lang}</Badge>
                  <StatusBadge status={ext.status} />
                </div>
                <div className="mt-1.5 flex items-center gap-1.5">
                  {ext.domains.map((domain) => (
                    <span key={domain} className="inline-flex items-center gap-1 text-[10px] text-muted-foreground/40">
                      <ExternalLink size={9} />
                      {domain}
                    </span>
                  ))}
                </div>
                {ext.features.length > 0 && (
                  <div className="mt-1.5 flex items-center gap-1">
                    {ext.features.map((feat) => (
                      <Badge key={feat} variant="info" size="sm">{feat}</Badge>
                    ))}
                  </div>
                )}
                {ext.localVersionId != null && ext.status === "update_available" && (
                  <p className="mt-1 text-[10px] text-muted-foreground/60">
                    v{ext.localVersionId != null ? formatVersionId(ext.localVersionId) : '?'} → v{formatVersionId(ext.remoteVersionId)}
                  </p>
                )}
              </div>

              {/* Action */}
              <div className="flex items-center gap-2">
                {ext.status === "not_installed" && (
                  <Button
                    variant="primary"
                    size="sm"
                    icon={<Download size={14} />}
                    onClick={() => onInstall(ext)}
                    disabled={isBusy}
                  >
                    {isThisInstalling ? t("extensions.installing") : t("extensions.install")}
                  </Button>
                )}
                {ext.status === "update_available" && (
                  <Button
                    variant="primary"
                    size="sm"
                    icon={<ArrowUpCircle size={14} />}
                    onClick={() => onUpdate(ext)}
                    disabled={isBusy}
                  >
                    {isThisUpdating ? t("extensions.updating") : t("extensions.updateAvailable")}
                  </Button>
                )}
                {ext.status === "up_to_date" && (
                  <Badge variant="success" size="sm">
                    <CheckCircle2 size={10} />
                  </Badge>
                )}
              </div>
            </div>
          </Card>
        );
      })}

      {updates.length === 0 && (
        <EmptyState
          icon={<Download size={28} strokeWidth={1.5} className="text-muted-foreground/50" />}
          title={t("extensions.catalogEmpty")}
        />
      )}
    </div>
  );
}
