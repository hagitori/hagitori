import {
  Puzzle,
  ExternalLink,
  Globe,
} from "lucide-react";
import { Button, Card, Badge, EmptyState, SkeletonCard, Select } from "@/components/ui";
import { useTranslation } from "@/hooks/useTranslation";

export interface InstalledTabProps {
  extensions: {
    id: string;
    name: string;
    version: string;
    lang: string;
    domains: string[];
    features: string[];
    languages: string[];
    icon?: string;
  }[];
  loading: boolean;
  extensionLangs: Record<string, string>;
  onLangChange: (id: string, lang: string) => void;
  onRemove: (id: string) => void;
}

export function InstalledTab({
  extensions,
  loading,
  extensionLangs,
  onLangChange,
  onRemove,
}: InstalledTabProps) {
  const { t } = useTranslation();
  if (loading) {
    return (
      <div className="flex flex-col gap-2">
        {[1, 2, 3].map((i) => (
          <SkeletonCard key={i} />
        ))}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {extensions.map((ext) => (
        <Card key={ext.id}>
          <div className="flex items-center gap-4">
            {/* Icon */}
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 overflow-hidden">
              {ext.icon ? (
                <img
                  src={ext.icon}
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
                <Badge variant="outline" size="sm">v{ext.version}</Badge>
                <Badge variant="default" size="sm">{ext.lang}</Badge>
              </div>
              <div className="mt-1.5 flex items-center gap-1.5">
                {ext.domains.map((domain) => (
                  <span key={domain} className="inline-flex items-center gap-1 text-[10px] text-muted-foreground/40">
                    <ExternalLink size={9} />
                    {domain}
                  </span>
                ))}
              </div>
              {ext.features && ext.features.length > 0 && (
                <div className="mt-1.5 flex items-center gap-1">
                  {ext.features.map((feat) => (
                    <Badge key={feat} variant="info" size="sm">{feat}</Badge>
                  ))}
                </div>
              )}
              {ext.languages && ext.languages.length > 0 && (
                <div className="mt-2 flex items-center gap-2">
                  <Globe size={12} className="shrink-0 text-muted-foreground/50" />
                  <Select
                    value={extensionLangs[ext.id] || ext.lang}
                    options={ext.languages}
                    onChange={(lang) => onLangChange(ext.id, lang)}
                    className="h-7 w-auto"
                  />
                  <span className="text-[10px] text-muted-foreground/40">
                    {t("extensions.selectLanguage")}
                  </span>
                </div>
              )}
            </div>

            {/* Actions */}
            <div className="flex items-center gap-1.5">
              <Button
                variant="outline"
                size="sm"
                onClick={() => onRemove(ext.id)}
                className="hover:border-[hsl(var(--danger))]/30 hover:bg-[hsl(var(--danger))]/5 hover:text-[hsl(var(--danger))]"
              >
                {t("extensions.remove")}
              </Button>
            </div>
          </div>
        </Card>
      ))}

      {extensions.length === 0 && (
        <EmptyState
          icon={<Puzzle size={28} strokeWidth={1.5} className="text-muted-foreground/50" />}
          title={t("extensions.empty")}
        />
      )}
    </div>
  );
}
