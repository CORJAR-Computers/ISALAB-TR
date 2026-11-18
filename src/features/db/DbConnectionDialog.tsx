import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { relaunch } from "@tauri-apps/plugin-process";
import { Loader2, PlugZap, Save, Server } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import type { DbTestResult } from "@/bindings";
import { api, getErrorMessage } from "@/lib/api";
import { DB_STATUS_LABEL, dbStatusVariant } from "@/lib/db-status";

const schema = z.object({
  host: z.string().trim().min(1, "Indica la IP o el nombre del servidor"),
  port: z
    .string()
    .regex(/^\d+$/, "Puerto inválido")
    .refine((v) => Number(v) >= 1 && Number(v) <= 65535, "Puerto entre 1 y 65535"),
  database: z
    .string()
    .trim()
    .min(1, "Indica el alias (ISALAB) o la ruta de la base de datos"),
  user: z.string().trim().min(1, "Indica el usuario de la base de datos"),
  password: z.string(),
});

type Values = z.infer<typeof schema>;

/**
 * Diálogo para apuntar la app al servidor Firebird de la clínica.
 *
 * Se abre desde el login y desde el banner porque, si la base no está
 * configurada o el servidor no responde, no hay forma de iniciar sesión.
 * Guarda `db-connection.json` (contraseña cifrada con DPAPI) y reinicia la
 * app para aplicar la nueva conexión.
 */
export function DbConnectionDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<DbTestResult | null>(null);
  const [hasPassword, setHasPassword] = useState(false);
  const [source, setSource] = useState<string>("file");

  const { data: info } = useQuery({
    queryKey: ["db-connection"],
    queryFn: api.getDbConnection,
    enabled: open,
    retry: false,
  });

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      host: "",
      port: "3050",
      database: "ISALAB",
      user: "ISALAB_APP",
      password: "",
    },
  });

  useEffect(() => {
    if (!info) return;
    setHasPassword(info.hasPassword);
    setSource(info.source);
    form.reset({
      host: info.host,
      port: String(info.port),
      database: info.database || "ISALAB",
      user: info.user || "ISALAB_APP",
      password: "",
    });
  }, [info, form]);

  const onSubmit = async (vals: Values) => {
    setSaving(true);
    try {
      const target = await api.saveDbConnection({
        host: vals.host,
        port: Number(vals.port),
        database: vals.database,
        user: vals.user,
        password: vals.password,
      });
      toast.success("Conexión guardada", {
        description: `La aplicación se reiniciará para conectar con ${target}.`,
      });
      try {
        await relaunch();
      } catch {
        // Sin runtime Tauri (tests/navegador): basta con cerrar el diálogo.
        onOpenChange(false);
      }
    } catch (e) {
      toast.error("No se pudo guardar la conexión", {
        description: getErrorMessage(e),
      });
    } finally {
      setSaving(false);
    }
  };

  const runTest = async () => {
    const valid = await form.trigger();
    if (!valid) return;
    setTesting(true);
    setResult(null);
    try {
      const vals = form.getValues();
      const res = await api.testDbConnection({
        host: vals.host,
        port: Number(vals.port),
        database: vals.database,
        user: vals.user,
        password: vals.password,
      });
      setResult(res);
      if (res.status === "connected") {
        toast.success("Conexión correcta", { description: res.message });
      }
    } catch (e) {
      toast.error("No se pudo probar la conexión", {
        description: getErrorMessage(e),
      });
    } finally {
      setTesting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Server className="size-4 text-primary" />
            Servidor Firebird
          </DialogTitle>
          <DialogDescription>
            Dirección del servidor central de la clínica. Los tres equipos deben
            apuntar al mismo host y a la misma base de datos.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-3">
              <FormField
                control={form.control}
                name="host"
                render={({ field }) => (
                  <FormItem className="sm:col-span-2">
                    <FormLabel>Servidor (IP o nombre)</FormLabel>
                    <FormControl>
                      <Input placeholder="192.168.1.10" autoFocus {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="port"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Puerto</FormLabel>
                    <FormControl>
                      <Input inputMode="numeric" placeholder="3050" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="database"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Base de datos</FormLabel>
                    <FormControl>
                      <Input placeholder="ISALAB" {...field} />
                    </FormControl>
                    <FormDescription>
                      Alias definido en el servidor.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="user"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Usuario</FormLabel>
                    <FormControl>
                      <Input placeholder="ISALAB_APP" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <FormField
              control={form.control}
              name="password"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Contraseña</FormLabel>
                  <FormControl>
                    <Input
                      type="password"
                      autoComplete="off"
                      placeholder={
                        hasPassword ? "•••••••• (sin cambios)" : "Contraseña del usuario"
                      }
                      {...field}
                    />
                  </FormControl>
                  <FormDescription>
                    Se guarda cifrada con DPAPI en este equipo; nunca viaja en
                    claro.
                    {source === "environment" &&
                      " Hay variables de entorno ISALAB_DB_* que tienen prioridad sobre este archivo."}
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            {result && (
              <div className="bg-muted/40 rounded-lg border p-3 text-sm">
                <div className="flex items-center gap-2">
                  <Badge variant={dbStatusVariant(result.status)}>
                    {DB_STATUS_LABEL[result.status]}
                  </Badge>
                  <span className="text-muted-foreground text-xs">
                    {result.latencyMs} ms
                    {result.schemaVersion > 0 &&
                      ` · schema v${result.schemaVersion}`}
                  </span>
                </div>
                <p className="text-muted-foreground mt-1.5 text-xs leading-relaxed">
                  {result.message}
                </p>
              </div>
            )}

            <DialogFooter className="gap-2 sm:justify-between">
              <Button
                type="button"
                variant="outline"
                onClick={runTest}
                disabled={testing || saving}
              >
                {testing ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <PlugZap className="size-4" />
                )}
                Probar conexión
              </Button>
              <Button type="submit" disabled={saving || testing}>
                {saving ? <Loader2 className="animate-spin" /> : <Save className="size-4" />}
                Guardar y reiniciar
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
