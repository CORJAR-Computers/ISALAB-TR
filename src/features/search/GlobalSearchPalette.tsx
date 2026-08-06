import { useEffect, useRef, useState } from "react";
import {
  CornerDownLeft,
  FlaskConical,
  Receipt,
  Scissors,
  Search,
  Users,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { useGlobalSearch } from "@/hooks/use-queries";
import { useUiStore } from "@/stores/ui-store";
import { cn } from "@/lib/utils";
import type { GlobalSearchResult } from "@/bindings";

const KIND_META: Record<
  string,
  { icon: typeof Users; label: string; tone: string }
> = {
  patient: { icon: Users, label: "Paciente", tone: "bg-primary/10 text-primary" },
  sample: { icon: FlaskConical, label: "Muestra", tone: "bg-warning/15 text-warning" },
  invoice: { icon: Receipt, label: "Factura", tone: "bg-success/15 text-success" },
  surgery: { icon: Scissors, label: "Cirugía", tone: "bg-destructive/10 text-destructive" },
};

/**
 * Paleta de búsqueda global (Ctrl+K / ⌘K): salta a un paciente, muestra,
 * factura o cirugía por código o nombre, con navegación por teclado.
 */
export function GlobalSearchPalette() {
  const open = useUiStore((s) => s.searchOpen);
  const openSearch = useUiStore((s) => s.openSearch);
  const closeSearch = useUiStore((s) => s.closeSearch);
  const navigate = useUiStore((s) => s.navigate);
  const setActivePatient = useUiStore((s) => s.setActivePatient);
  const requestEntity = useUiStore((s) => s.requestEntity);

  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);

  // Debounce de 250 ms para no golpear la BD en cada tecla.
  useEffect(() => {
    const t = setTimeout(() => setDebounced(query), 250);
    return () => clearTimeout(t);
  }, [query]);

  const { data, isLoading } = useGlobalSearch(debounced, open);

  // Al cerrar se limpia el estado; al abrir, foco en el input.
  useEffect(() => {
    if (!open) {
      setQuery("");
      setDebounced("");
      setSelected(0);
    }
  }, [open]);

  // Atajo global Ctrl+K / ⌘K.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        if (open) closeSearch();
        else openSearch();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, openSearch, closeSearch]);

  const results = data ?? [];

  // Nueva lista de resultados → volver al primero.
  useEffect(() => {
    setSelected(0);
  }, [debounced]);

  const go = (r: GlobalSearchResult) => {
    closeSearch();
    switch (r.kind) {
      case "patient":
        setActivePatient(r.id);
        navigate("clinical-history");
        break;
      case "sample":
        navigate("samples");
        requestEntity("sample", r.id);
        break;
      case "invoice":
        navigate("invoices");
        requestEntity("invoice", r.id);
        break;
      case "surgery":
        navigate("surgeries");
        requestEntity("surgery", r.id);
        break;
    }
  };

  const onInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (results.length === 0) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const r = results[selected];
      if (r) go(r);
    }
  };

  const trimmed = query.trim();

  return (
    <Dialog open={open} onOpenChange={(o) => !o && closeSearch()}>
      <DialogContent
        showCloseButton={false}
        className="gap-0 overflow-hidden p-0 sm:max-w-xl"
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          inputRef.current?.focus();
        }}
      >
        <DialogTitle className="sr-only">Buscar en ISALAB</DialogTitle>
        {/* Campo de búsqueda */}
        <div className="flex items-center gap-2.5 border-b px-4">
          <Search className="text-muted-foreground size-4 shrink-0" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onInputKeyDown}
            placeholder="Buscar paciente, muestra, factura o cirugía…"
            className="h-12 min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
          <kbd className="text-muted-foreground hidden rounded border bg-muted px-1.5 py-0.5 text-[10px] font-medium sm:block">
            Esc
          </kbd>
        </div>

        {/* Resultados */}
        <div className="max-h-80 overflow-y-auto p-1.5">
          {trimmed.length === 0 && (
            <p className="text-muted-foreground px-3 py-8 text-center text-xs">
              Escribe un código (PAC-, M-, FAC-) o un nombre para buscar en
              todo el sistema.
            </p>
          )}

          {trimmed.length > 0 && isLoading && (
            <div className="space-y-1 p-1">
              {[0, 1, 2].map((i) => (
                <Skeleton key={i} className="h-11 w-full" />
              ))}
            </div>
          )}

          {trimmed.length > 0 && !isLoading && results.length === 0 && (
            <p className="text-muted-foreground px-3 py-8 text-center text-xs">
              Sin resultados para “{trimmed}”.
            </p>
          )}

          {results.length > 0 && (
            <ul className="space-y-0.5">
              {results.map((r, i) => {
                const meta = KIND_META[r.kind] ?? KIND_META.patient;
                const Icon = meta.icon;
                const active = selected === i;
                return (
                  <li key={`${r.kind}-${r.id}`}>
                    <button
                      type="button"
                      onMouseEnter={() => setSelected(i)}
                      onClick={() => go(r)}
                      className={cn(
                        "flex w-full items-center gap-3 rounded-lg px-2.5 py-2 text-left transition-colors",
                        active
                          ? "bg-accent text-accent-foreground"
                          : "hover:bg-accent/60",
                      )}
                    >
                      <span
                        className={cn(
                          "flex size-8 shrink-0 items-center justify-center rounded-md",
                          meta.tone,
                        )}
                      >
                        <Icon className="size-4" />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="flex items-baseline gap-2">
                          <span className="truncate text-sm font-medium">
                            {r.title}
                          </span>
                          {r.code && (
                            <span className="font-mono text-[10px] text-muted-foreground">
                              {r.code}
                            </span>
                          )}
                        </span>
                        <span className="text-muted-foreground block truncate text-xs">
                          {r.subtitle}
                        </span>
                      </span>
                      <span className="text-muted-foreground flex shrink-0 items-center gap-1 text-[10px] tracking-wide uppercase">
                        {meta.label}
                        {active && <CornerDownLeft className="size-3" />}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>

        {/* Pie con atajos */}
        <div className="text-muted-foreground flex items-center gap-3 border-t px-4 py-2 text-[10px]">
          <span className="flex items-center gap-1">
            <kbd className="rounded border bg-muted px-1 py-0.5">↑↓</kbd> navegar
          </span>
          <span className="flex items-center gap-1">
            <kbd className="rounded border bg-muted px-1 py-0.5">↵</kbd> abrir
          </span>
          <span className="ml-auto hidden sm:block">
            Busca por código o nombre en pacientes, muestras, facturas y cirugías
          </span>
        </div>
      </DialogContent>
    </Dialog>
  );
}
