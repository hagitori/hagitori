import { useState, useEffect } from "react";
import { Globe, Link, X } from "lucide-react";
import { Button, Input, useToast } from "@/components/ui";
import { useTranslation } from "@/hooks/useTranslation";
import { setCatalogUrl, getConfig } from "@/lib/tauri";

export interface AddRepoModalProps {
  onClose: () => void;
  onSuccess: () => void;
}

export function AddRepoModal({ onClose, onSuccess }: AddRepoModalProps) {
  const { t } = useTranslation();
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const toast = useToast();

  // load current URL when opening the modal
  useEffect(() => {
    getConfig().then((config) => {
      const current = config["extensions_catalog_url"];
      if (current) setUrl(current);
    });
  }, []);

  const handleConfirm = async () => {
    // validate URL
    if (!url.trim()) {
      setError(t("extensions.addRepoInvalidUrl"));
      return;
    }

    if (!url.trim().endsWith("catalog.json") && !url.trim().endsWith("catalog.min.json")) {
      setError(t("extensions.addRepoInvalidUrl"));
      return;
    }

    try {
      new URL(url.trim());
    } catch {
      setError(t("extensions.addRepoInvalidUrl"));
      return;
    }

    setLoading(true);
    setError("");

    try {
      await setCatalogUrl(url.trim());
      toast.success(t("extensions.addRepoSuccess"));
      onSuccess();
    } catch (err) {
      setError(String(err));
      toast.error(t("extensions.addRepoError"));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-md rounded-xl border border-border bg-card p-6 shadow-2xl">
        {/* Header */}
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Link size={18} className="text-primary" />
            <h2 className="text-lg font-semibold text-foreground">
              {t("extensions.addRepoTitle")}
            </h2>
          </div>
          <button
            onClick={onClose}
            className="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Description */}
        <p className="mb-4 text-sm text-muted-foreground">
          {t("extensions.addRepoDescription")}
        </p>

        {/* Input */}
        <Input
          value={url}
          onChange={(e) => {
            setUrl(e.target.value);
            setError("");
          }}
          placeholder={t("extensions.addRepoPlaceholder")}
          icon={<Globe size={14} />}
          error={error}
          className="h-9"
          onKeyDown={(e) => {
            if (e.key === "Enter") handleConfirm();
          }}
        />

        {/* Actions */}
        <div className="mt-5 flex items-center justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={loading}>
            {t("extensions.addRepoCancel")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={handleConfirm}
            disabled={loading || !url.trim()}
            loading={loading}
          >
            {t("extensions.addRepoConfirm")}
          </Button>
        </div>
      </div>
    </div>
  );
}
