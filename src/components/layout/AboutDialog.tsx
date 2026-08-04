import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useUiStore } from "@/stores/ui-store";
import logoSidebar from "@/assets/logo_sidebar.png";
import {
  Cpu,
  Database,
  FileCheck2,
  Globe,
  HeartPulse,
  Info,
  ShieldCheck,
  Sparkles,
} from "lucide-react";

export function AboutDialog() {
  const aboutOpen = useUiStore((s) => s.aboutOpen);
  const setAboutOpen = useUiStore((s) => s.setAboutOpen);

  return (
    <Dialog open={aboutOpen} onOpenChange={setAboutOpen}>
      <DialogContent className="max-w-xl p-0 overflow-hidden rounded-2xl border shadow-2xl">
        {/* Encabezado con degradado y logo */}
        <div className="relative bg-gradient-to-br from-slate-900 via-primary/95 to-slate-900 p-6 text-white overflow-hidden">
          <div className="absolute -right-12 -top-12 size-40 rounded-full bg-emerald-500/10 blur-2xl pointer-events-none" />
          <div className="absolute -left-12 -bottom-12 size-40 rounded-full bg-blue-500/10 blur-2xl pointer-events-none" />

          <div className="flex items-center gap-4 relative z-10">
            <div className="flex h-16 w-44 items-center justify-center rounded-xl bg-white/10 p-2 backdrop-blur-md border border-white/15 shadow-inner">
              <img
                src={logoSidebar}
                alt="ISALAB"
                className="max-h-full w-full object-contain"
              />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <Badge variant="secondary" className="bg-emerald-500/20 text-emerald-300 border-emerald-500/30 text-[10px] uppercase font-bold tracking-wider">
                  Tauri v2 + Rust
                </Badge>
                <Badge variant="outline" className="text-white/80 border-white/20 text-[10px]">
                  v2.0.0
                </Badge>
              </div>
              <h2 className="text-xl font-bold tracking-tight mt-1 text-white">
                ISALAB
              </h2>
              <p className="text-xs text-white/80">
                Sistema de Gestión & Trazabilidad para Laboratorios Veterinarios
              </p>
            </div>
          </div>
        </div>

        <div className="p-6 space-y-5 text-sm">
          {/* Tarjeta de la Empresa */}
          <div className="rounded-xl border bg-gradient-to-r from-primary/5 via-emerald-500/5 to-transparent p-4 flex items-center justify-between gap-4">
            <div className="space-y-0.5">
              <span className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
                <Sparkles className="size-3.5 text-primary" /> Desarrollado por
              </span>
              <h3 className="text-base font-bold text-foreground tracking-tight">
                CORJAR Computers Solutions
              </h3>
              <p className="text-xs text-muted-foreground">
                Ingeniería de Software & Soluciones Tecnológicas de Alto Rendimiento
              </p>
            </div>
            <div className="shrink-0 flex items-center justify-center size-10 rounded-full bg-primary/10 text-primary">
              <Globe className="size-5" />
            </div>
          </div>

          {/* Ficha de arquitectura técnica */}
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-lg border p-3 bg-card space-y-1">
              <div className="flex items-center gap-2 text-xs font-semibold text-foreground">
                <Cpu className="size-4 text-emerald-600" /> Shell & Core
              </div>
              <p className="text-xs text-muted-foreground">
                Tauri v2 + Rust (x86_64 Desktop Nativo)
              </p>
            </div>

            <div className="rounded-lg border p-3 bg-card space-y-1">
              <div className="flex items-center gap-2 text-xs font-semibold text-foreground">
                <Database className="size-4 text-blue-600" /> Base de Datos
              </div>
              <p className="text-xs text-muted-foreground">
                Firebird 5.0 Embedded (Página 16 KB)
              </p>
            </div>

            <div className="rounded-lg border p-3 bg-card space-y-1">
              <div className="flex items-center gap-2 text-xs font-semibold text-foreground">
                <FileCheck2 className="size-4 text-purple-600" /> Documentos PDF
              </div>
              <p className="text-xs text-muted-foreground">
                printpdf (Server-side Rust, Formato Carta)
              </p>
            </div>

            <div className="rounded-lg border p-3 bg-card space-y-1">
              <div className="flex items-center gap-2 text-xs font-semibold text-foreground">
                <ShieldCheck className="size-4 text-amber-600" /> Seguridad & Auth
              </div>
              <p className="text-xs text-muted-foreground">
                Argon2id + CSP + RBAC Control de Roles
              </p>
            </div>
          </div>

          {/* Características clave */}
          <div className="space-y-2 border-t pt-4">
            <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider flex items-center gap-1.5">
              <HeartPulse className="size-3.5 text-rose-500" /> Capacidades Clínicas
            </h4>
            <ul className="text-xs text-muted-foreground space-y-1 list-disc pl-4">
              <li>Trazabilidad de muestras y validación analítica multiespecie por edad y sexo.</li>
              <li>Generación de informes de laboratorio, recetas, consentimientos, recibos y carnets de vacunación.</li>
              <li>Auditoría nativa de acciones de usuario y almacenamiento cifrado local.</li>
            </ul>
          </div>
        </div>

        {/* Pie del modal */}
        <DialogFooter className="bg-muted/40 px-6 py-3 border-t flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
          <p className="text-[11px] text-muted-foreground text-center sm:text-left">
            © 2026 <strong className="text-foreground font-semibold">CORJAR Computers Solutions</strong>. Todos los derechos reservados.
          </p>
          <Button size="sm" onClick={() => setAboutOpen(false)}>
            Cerrar
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
