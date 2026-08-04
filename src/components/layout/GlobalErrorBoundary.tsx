import { AlertOctagon, RefreshCw } from "lucide-react";
import type { FallbackProps } from "react-error-boundary";
import { Button } from "@/components/ui/button";

export function GlobalErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
  return (
    <div className="flex min-h-svh flex-col items-center justify-center p-6 text-center bg-background">
      <div className="bg-destructive/10 text-destructive mb-6 flex size-20 items-center justify-center rounded-3xl shadow-sm ring-1 ring-destructive/20">
        <AlertOctagon className="size-10" />
      </div>
      <h1 className="mb-2 text-2xl font-bold tracking-tight">
        Ocurrió un error inesperado
      </h1>
      <p className="text-muted-foreground mb-8 max-w-md text-sm leading-relaxed">
        La aplicación encontró un problema crítico y no pudo continuar. Puedes intentar recargar la aplicación para solucionar el problema.
      </p>
      
      <div className="bg-muted/50 mb-8 w-full max-w-xl overflow-auto rounded-lg border p-4 text-left font-mono text-xs">
        <span className="font-semibold text-destructive">Error: </span> 
        {(error as Error).message}
      </div>

      <div className="flex flex-col sm:flex-row gap-3">
        <Button onClick={resetErrorBoundary} className="gap-2 w-full sm:w-auto">
          <RefreshCw className="size-4" />
          Reintentar
        </Button>
        <Button 
          variant="outline" 
          onClick={() => window.location.reload()}
          className="w-full sm:w-auto"
        >
          Recargar completamente
        </Button>
      </div>
    </div>
  );
}
