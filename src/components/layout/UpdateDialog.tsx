import { Download } from "lucide-react";
import type { Update } from "@tauri-apps/plugin-updater";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { UpdateProgress } from "@/hooks/use-app-updater";

interface UpdateDialogProps {
  update: Update;
  downloading: boolean;
  progress: UpdateProgress | null;
  onInstall: () => void;
  onDismiss: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024)),
  );
  return `${(bytes / 1024 ** i).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function UpdateDialog({
  update,
  downloading,
  progress,
  onInstall,
  onDismiss,
}: UpdateDialogProps) {
  const percent =
    progress && progress.contentLength > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.contentLength) * 100))
      : 0;

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !downloading) onDismiss();
      }}
    >
      <DialogContent className="sm:max-w-md" showCloseButton={!downloading}>
        <DialogHeader>
          <DialogTitle>Nueva versión disponible</DialogTitle>
          <DialogDescription>
            ISALAB v{update.version} ya está lista para instalar.
          </DialogDescription>
        </DialogHeader>

        {update.body && (
          <div className="bg-muted/50 max-h-40 overflow-y-auto rounded-lg border p-3 text-xs leading-relaxed whitespace-pre-wrap">
            {update.body}
          </div>
        )}

        {downloading ? (
          <div className="space-y-2">
            <div className="bg-muted h-2 w-full overflow-hidden rounded-full">
              <div
                className="bg-primary h-full rounded-full transition-all duration-200"
                style={{ width: `${percent}%` }}
              />
            </div>
            <p className="text-muted-foreground text-xs">
              {percent}%{" "}
              {progress?.contentLength
                ? `de ${formatBytes(progress.contentLength)}`
                : ""}{" "}
              · No cierres la aplicación
            </p>
          </div>
        ) : (
          <p className="text-muted-foreground text-sm">
            La actualización se descargará e instalará automáticamente. La
            aplicación se reiniciará al terminar.
          </p>
        )}

        <DialogFooter>
          {!downloading && (
            <>
              <Button variant="outline" onClick={onDismiss}>
                Más tarde
              </Button>
              <Button onClick={onInstall}>
                <Download className="size-4" />
                Actualizar ahora
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
