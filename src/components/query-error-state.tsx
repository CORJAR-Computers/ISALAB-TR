import { AlertTriangle, RotateCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { getErrorMessage } from "@/lib/api";
import { cn } from "@/lib/utils";

type QueryErrorStateProps = {
  /** Qué no se pudo cargar, en minúsculas: "la agenda de citas", "los usuarios". */
  what: string;
  error: unknown;
  onRetry: () => void;
  /** Aclaración opcional sobre qué hacer (revisar la base, reintentar…). */
  hint?: string;
  /** Versión reducida para huecos dentro de una Card o una tabla. */
  compact?: boolean;
  className?: string;
};

/**
 * Estado de error de una consulta, con reintento.
 *
 * Existía el mismo bloque copiado a mano en varias páginas: un fallo de la
 * consulta se veía igual que "no hay datos" (o se quedaba cargando para
 * siempre). Centralizarlo garantiza que siempre haya mensaje real, rol de
 * alerta para lectores de pantalla y una vía de reintento.
 */
export function QueryErrorState({
  what,
  error,
  onRetry,
  hint,
  compact,
  className,
}: QueryErrorStateProps) {
  return (
    <div
      role="alert"
      className={cn(
        "flex flex-col items-center gap-2 text-center",
        compact
          ? "px-3 py-6"
          : "border-destructive/30 from-destructive/5 rounded-xl border bg-gradient-to-b to-transparent px-6 py-10",
        className,
      )}
    >
      <AlertTriangle className="text-destructive size-6" aria-hidden="true" />
      <p className="text-sm font-semibold">No se pudo cargar {what}</p>
      {hint && (
        <p className="text-muted-foreground max-w-md text-xs">{hint}</p>
      )}
      <p className="text-muted-foreground max-w-md font-mono text-xs break-words">
        {getErrorMessage(error)}
      </p>
      <Button variant="outline" size="sm" className="mt-1 gap-2" onClick={onRetry}>
        <RotateCw className="size-3.5" aria-hidden="true" />
        Reintentar
      </Button>
    </div>
  );
}
