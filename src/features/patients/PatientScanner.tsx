import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
} from "react";
import { Loader2, ScanLine, UserX } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { usePatientByCode } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";

interface PatientScannerProps {
  /** Se invoca con el id del paciente encontrado (abre su ficha). */
  onFound: (patientId: number) => void;
}

/**
 * Escáner de código de paciente para la mesa de pacientes.
 *
 * Los lectores de código de barras se comportan como un teclado: escriben el
 * código y "pulsan Enter". Por eso la búsqueda con `usePatientByCode` solo se
 * dispara al pulsar Enter (no por cada tecla) y, al encontrar el paciente, se
 * abre su ficha automáticamente. Si el código no existe, se muestra un aviso
 * "no encontrado" y el input queda listo para la siguiente lectura.
 */
export function PatientScanner({ onFound }: PatientScannerProps) {
  const [code, setCode] = useState("");
  const [searched, setSearched] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Ref estable de onFound: evita que el efecto se dispare por identidad.
  const onFoundRef = useRef(onFound);
  useEffect(() => {
    onFoundRef.current = onFound;
  });

  const { data: patient, isFetching, isError, error } =
    usePatientByCode(searched);

  const notFound =
    searched !== null && !isFetching && !isError && patient === null;

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    const trimmed = code.trim();
    // Sin guard de isFetching: una segunda lectura rápida (p. ej. tag
    // equivocado → correcto) debe reemplazar a la búsqueda en vuelo.
    if (!trimmed) return;
    setSearched(trimmed);
    // Limpia el input tras la lectura: los escáneres "pegan" el código, y sin
    // esto las lecturas consecutivas se concatenarían.
    setCode("");
    inputRef.current?.focus();
  };

  // Al resolverse la búsqueda con un paciente, abre su ficha y reinicia.
  useEffect(() => {
    if (!patient) return;
    onFoundRef.current(patient.id);
    setSearched(null);
    inputRef.current?.focus();
  }, [patient]);

  return (
    <div className="rounded-xl border bg-card p-3 shadow-sm">
      <div className="mb-2 flex items-center gap-2">
        <span className="bg-primary/10 text-primary flex size-6 items-center justify-center rounded-md">
          <ScanLine className="size-3.5" />
        </span>
        <p className="text-muted-foreground text-xs font-medium tracking-wide uppercase">
          Escáner de paciente
        </p>
      </div>

      <form onSubmit={handleSubmit} className="flex items-center gap-2">
        <div className="relative flex-1">
          <Input
            ref={inputRef}
            autoFocus
            value={code}
            onChange={(e) => {
              setCode(e.target.value);
              // Al teclear se descarta el aviso anterior de "no encontrado".
              if (searched !== null) setSearched(null);
            }}
            placeholder="Escanea o escribe el código (PAC-…) y pulsa Enter"
            className="pr-9 font-mono"
            aria-label="Código de paciente"
          />
          {isFetching && (
            <Loader2 className="text-muted-foreground absolute top-1/2 right-3 size-4 -translate-y-1/2 animate-spin" />
          )}
        </div>
        <Button type="submit" size="sm" disabled={!code.trim() || isFetching}>
          Buscar
        </Button>
      </form>

      {notFound && (
        <p
          role="alert"
          className="text-destructive mt-2 flex items-center gap-1.5 text-xs"
        >
          <UserX className="size-3.5" />
          No se encontró ningún paciente con el código «{searched}».
        </p>
      )}
      {isError && (
        <p
          role="alert"
          className="text-destructive mt-2 flex items-center gap-1.5 text-xs"
        >
          No se pudo consultar el código: {getErrorMessage(error)}
        </p>
      )}
    </div>
  );
}
