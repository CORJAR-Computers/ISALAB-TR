import { useEffect, useRef, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Building2, CreditCard, FileKey2, ImageUp, Loader2, PenLine, Save, X, Bot, DatabaseBackup, Wifi, ShieldAlert, Mail } from "lucide-react";
import { usePermissions } from "@/hooks/use-permissions";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
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
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import {
  useClinicSettings,
  useDeleteClinicLogo,
  useDeletePkcs12,
  useImportClinicLogo,
  useImportPkcs12,
  useSaveClinicSettings,
  useSecondaryLogos,
  useImportSecondaryLogo,
  useDeleteSecondaryLogo,
  useTestSmtpConnection,
} from "@/hooks/use-queries";
import { AnalyzerManagementCard } from "@/features/settings/AnalyzerManagementCard";
import { api, getErrorMessage } from "@/lib/api";
import type { ClinicSettings } from "@/bindings";
import { Trash2 } from "lucide-react";

const schema = z.object({
  clinicName: z.string().min(1, "El nombre de la clínica es obligatorio"),
  clinicNit: z.string().min(1, "El NIT es obligatorio"),
  address: z.string().optional(),
  phone: z.string().optional(),
  city: z.string().optional(),
  taxRate: z.coerce.number().min(0).max(100, "IVA entre 0 y 100"),
  currency: z.string().min(1, "Moneda obligatoria"),
  signatureMode: z.enum(["GRAPHIC", "DIGITAL"]),
  vetName: z.string().optional(),
  vetLicense: z.string().optional(),
  groqApiKey: z.string().optional(),
  pkcs12Password: z.string().optional(),
  tagline: z.string().optional(),
  groomingDuration: z.string().optional(),
  receptionTicketPrefix: z.string().optional(),
  smtpHost: z.string().optional(),
  smtpPort: z.coerce.number().int().min(1).max(65535).optional(),
  smtpTls: z.enum(["NONE", "STARTTLS", "TLS"]),
  smtpUsername: z.string().optional(),
  smtpPassword: z.string().optional(),
  smtpFrom: z.string().optional(),
});

type Values = z.infer<typeof schema>;

const toNullable = (v: unknown) =>
  typeof v === "string" && v.trim() !== "" ? v.trim() : null;

export function SettingsPage() {
  const { data: settings, isLoading } = useClinicSettings();
  const save = useSaveClinicSettings();
  const importLogo = useImportClinicLogo();
  const deleteLogo = useDeleteClinicLogo();
  const importPkcs12 = useImportPkcs12();
  const deletePkcs12 = useDeletePkcs12();
  const { data: secondaryLogos } = useSecondaryLogos();
  const importSecondaryLogo = useImportSecondaryLogo();
  const deleteSecondaryLogo = useDeleteSecondaryLogo();
  const { isAdmin } = usePermissions();

  const [backingUp, setBackingUp] = useState(false);
  const [testingGroq, setTestingGroq] = useState(false);
  const [testingSmtp, setTestingSmtp] = useState(false);
  const [logoContentUrl, setLogoContentUrl] = useState<string | null>(null);
  const [secondaryLogoContentUrls, setSecondaryLogoContentUrls] = useState<
    Record<number, string>
  >({});
  const loadedLogoIds = useRef<Set<number>>(new Set());
  const testSmtpMutation = useTestSmtpConnection();

  const form = useForm<z.input<typeof schema>, unknown, z.output<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: {
      clinicName: "",
      clinicNit: "",
      address: "",
      phone: "",
      city: "",
      taxRate: 19,
      currency: "COP",
      signatureMode: "GRAPHIC",
      vetName: "",
      vetLicense: "",
      groqApiKey: "",
      pkcs12Password: "",
      smtpHost: "",
      smtpPort: undefined,
      smtpTls: "STARTTLS",
      smtpUsername: "",
      smtpPassword: "",
      smtpFrom: "",
      tagline: "",
      groomingDuration: "",
      receptionTicketPrefix: "",
    },
  });

  // Carga los valores guardados cuando llegan del backend. `keepDirtyValues`
  // evita que un refetch (p. ej. la invalidación tras importar el logo o el
  // certificado) borre ediciones sin guardar, como la contraseña PKCS#12.
  useEffect(() => {
    if (settings) {
      form.reset({
        clinicName: settings.clinicName,
        clinicNit: settings.clinicNit,
        address: settings.address ?? "",
        phone: settings.phone ?? "",
        city: settings.city ?? "",
        taxRate: settings.taxRate ?? 19,
        currency: settings.currency,
        signatureMode:
          settings.signatureMode === "DIGITAL" ? "DIGITAL" : "GRAPHIC",
        vetName: settings.vetName ?? "",
        vetLicense: settings.vetLicense ?? "",
        groqApiKey: settings.groqApiKey ?? "",
        pkcs12Password: "",
        smtpHost: settings.smtpHost ?? "",
        smtpPort: settings.smtpPort ?? undefined,
        smtpTls: settings.smtpTls === "TLS" ? "TLS" : settings.smtpTls === "NONE" ? "NONE" : "STARTTLS",
        smtpUsername: settings.smtpUsername ?? "",
        smtpPassword: settings.smtpPassword ?? "",
        smtpFrom: settings.smtpFrom ?? "",
        tagline: settings.tagline ?? "",
        groomingDuration:
          settings.groomingDefaultDuration != null
            ? String(settings.groomingDefaultDuration)
            : "",
        receptionTicketPrefix: settings.receptionTicketPrefix ?? "",
      }, { keepDirtyValues: true });
    }
  }, [settings, form]);

  /** Selecciona un archivo de imagen y lo copia a la carpeta de la app. */
  const pickLogo = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: "Seleccionar logo de la clínica",
        filters: [
          { name: "Imágenes", extensions: ["png", "jpg", "jpeg", "webp"] },
        ],
      });
      if (!selected || Array.isArray(selected)) return;
      await importLogo.mutateAsync(selected);
      toast.success("Logo seleccionado", {
        description: "El logo se almacena en la base de datos y estará disponible en todos los PCs.",
      });
    } catch (e) {
      toast.error("No se pudo cargar el logo", {
        description: getErrorMessage(e),
      });
    }
  };

  const clearLogo = async () => {
    if (!window.confirm("¿Seguro que deseas eliminar el logo de la clínica?")) return;
    try {
      await deleteLogo.mutateAsync();
      toast.success("Logo eliminado", {
        description: "Los reportes volverán a usar el logo de ISALAB.",
      });
    } catch (e) {
      toast.error("No se pudo eliminar el logo", {
        description: getErrorMessage(e),
      });
    }
  };

  const pickSecondaryLogo = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: "Seleccionar logo secundario",
        filters: [{ name: "Imágenes", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (!selected || Array.isArray(selected)) return;
      
      const name = window.prompt("Nombre para este logo (ej. Logo Falso, Proyecto X):");
      if (!name || name.trim() === "") {
        return;
      }

      await importSecondaryLogo.mutateAsync({ name, sourcePath: selected });
      toast.success("Logo secundario añadido");
    } catch (e) {
      toast.error("No se pudo cargar el logo", {
        description: getErrorMessage(e),
      });
    }
  };

  const removeSecondaryLogo = async (id: number) => {
    if (!window.confirm("¿Seguro que deseas eliminar este logo secundario?")) return;
    try {
      await deleteSecondaryLogo.mutateAsync(id);
      toast.success("Logo eliminado");
    } catch (e) {
      toast.error("No se pudo eliminar", { description: getErrorMessage(e) });
    }
  };

  /** Precarga el contenido del logo principal como data URL para la vista previa. */
  useEffect(() => {
    if (!settings) return;
    (async () => {
      const bytes = await api.getClinicLogoContent();
      if (bytes) {
        const base64 = btoa(String.fromCharCode(...bytes));
        setLogoContentUrl(`data:image/png;base64,${base64}`);
      } else {
        setLogoContentUrl(null);
      }
    })();
  }, [settings]);

  /** Precarga el contenido de los logos secundarios como data URL. */
  useEffect(() => {
    if (!secondaryLogos) return;
    const pending: Promise<void>[] = [];
    for (const logo of secondaryLogos) {
      if (loadedLogoIds.current.has(logo.id)) continue;
      loadedLogoIds.current.add(logo.id);
      pending.push(
        (async () => {
          try {
            const bytes = await api.getSecondaryLogoContent(logo.id);
            if (bytes) {
              const base64 = btoa(String.fromCharCode(...bytes));
              setSecondaryLogoContentUrls((prev) => ({
                ...prev,
                [logo.id]: `data:image/png;base64,${base64}`,
              }));
            }
          } catch {
            // Ignorar errores individuales.
          }
        })(),
      );
    }
    void Promise.all(pending);
  }, [secondaryLogos]);

  const onSubmit = async (values: Values) => {
    const input: ClinicSettings = {
      clinicName: values.clinicName.trim(),
      clinicNit: values.clinicNit.trim(),
      address: toNullable(values.address),
      phone: toNullable(values.phone),
      city: toNullable(values.city),
      taxRate: values.taxRate,
      currency: values.currency.trim(),
      signatureMode: values.signatureMode,
      logo: settings?.logo ?? null,
      certificate: settings?.certificate ?? null,
      vetName: values.vetName?.trim() ?? "",
      vetLicense: toNullable(values.vetLicense),
      groqApiKey: toNullable(values.groqApiKey),
      // La contraseña solo viaja al backend para validarla y guardarla en
      // memoria; nunca se persiste. Se limpia tras guardar.
      pkcs12Password: toNullable(values.pkcs12Password),
      smtpHost: toNullable(values.smtpHost),
      smtpPort: values.smtpPort ?? null,
      smtpTls: values.smtpTls,
      smtpUsername: toNullable(values.smtpUsername),
      // El marcador de secreto redactado se reenvía tal cual para que el
      // backend conserve la contraseña almacenada si no se editó.
      smtpPassword:
        values.smtpPassword && values.smtpPassword !== ""
          ? values.smtpPassword.trim()
          : null,
      smtpFrom: toNullable(values.smtpFrom),
      tagline: toNullable(values.tagline),
      groomingDefaultDuration: (() => {
        const raw = values.groomingDuration?.trim();
        if (!raw) return null;
        const n = Number(raw);
        return Number.isFinite(n) && n > 0 ? Math.round(n) : null;
      })(),
      receptionTicketPrefix: toNullable(values.receptionTicketPrefix),
    };
    try {
      await save.mutateAsync(input);
      // Limpia la contraseña del formulario: ya se validó y quedó en memoria.
      form.setValue("pkcs12Password", "", { shouldDirty: true });
      toast.success("Configuración guardada", {
        description:
          "Los reportes PDF y la facturación usarán estos datos.",
      });
    } catch (e) {
      toast.error("No se pudo guardar", { description: getErrorMessage(e) });
    }
  };

  /** Selecciona un certificado PKCS#12, lo valida y lo copia a la app. */
  const pickPkcs12 = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: "Seleccionar certificado digital (PKCS#12)",
        filters: [
          { name: "Certificados PKCS#12", extensions: ["p12", "pfx"] },
        ],
      });
      if (!selected || Array.isArray(selected)) return;

      const password = form.getValues("pkcs12Password") ?? "";
      if (!password) {
        toast.error("Falta la contraseña del certificado", {
          description: "Ingresa la contraseña del .p12 antes de importarlo.",
        });
        return;
      }
      const stored = await importPkcs12.mutateAsync({
        sourcePath: selected,
        password,
      });
      toast.success("Certificado importado", {
        description: `Almacenado como "${stored.fileName ?? stored.kind}". Guarda la configuración para aplicar la firma digital.`,
      });
    } catch (e) {
      toast.error("No se pudo importar el certificado", {
        description: getErrorMessage(e),
      });
    }
  };

  const clearPkcs12 = async () => {
    if (!window.confirm("¿Seguro que deseas eliminar el certificado digital?")) return;
    try {
      await deletePkcs12.mutateAsync();
      form.setValue("pkcs12Password", "");
      toast.success("Certificado eliminado", {
        description: "Los reportes volverán a usar el modo de firma configurado.",
      });
    } catch (e) {
      toast.error("No se pudo eliminar el certificado", {
        description: getErrorMessage(e),
      });
    }
  };

  const testGroq = async () => {
    setTestingGroq(true);
    try {
      const result = await api.testGroqConnection();
      toast.success("Conexión con Groq exitosa", {
        description: result,
        duration: 6000,
      });
    } catch (e) {
      toast.error("No se pudo conectar con Groq", {
        description: getErrorMessage(e),
      });
    } finally {
      setTestingGroq(false);
    }
  };

  /** Prueba el SMTP guardado (NOOP) — primero guarda la configuración. */
  const testSmtp = async () => {
    setTestingSmtp(true);
    try {
      await testSmtpMutation.mutateAsync();
      toast.success("Conexión SMTP exitosa", {
        description: "El servidor de correo respondió correctamente.",
        duration: 6000,
      });
    } catch (e) {
      // El mensaje de error ya lo muestra el hook (toast).
      getErrorMessage(e);
    } finally {
      setTestingSmtp(false);
    }
  };

  const createBackup = async () => {
    try {
      const selected = await saveDialog({
        title: "Crear copia de seguridad",
        filters: [{ name: "Archivos ZIP", extensions: ["zip"] }],
        defaultPath: `ISALAB_Backup_${new Date().toISOString().split("T")[0]}.zip`,
      });
      if (!selected) return;

      setBackingUp(true);
      const destPath = await api.exportLocalConfig(selected);
      toast.success("Configuración exportada con éxito", {
        description: `Guardado en: ${destPath}`,
      });
    } catch (e) {
      toast.error("Error al crear copia de seguridad", {
        description: getErrorMessage(e),
      });
    } finally {
      setBackingUp(false);
    }
  };

  if (isLoading && !settings) {
    return (
      <div className="space-y-5">
        <Skeleton className="h-28 w-full" />
        <Skeleton className="h-28 w-full" />
        <Skeleton className="h-28 w-full" />
      </div>
    );
  }

  if (!isAdmin) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-20">
        <ShieldAlert className="size-10 text-muted-foreground" />
        <p className="text-muted-foreground text-sm">
          Solo los administradores pueden modificar la configuración.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div>
        <h2 className="text-xl font-semibold tracking-tight">
          Configuración
        </h2>
        <p className="text-muted-foreground text-sm">
          Datos de la clínica, facturación y firma de reportes.
        </p>
      </div>

      <Form {...form}>
        <form
          onSubmit={form.handleSubmit(onSubmit)}
          className="space-y-5"
        >
          {/* Datos de la clínica */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Building2 className="size-4 text-primary" />
                Datos de la clínica
              </CardTitle>
              <CardDescription>
                Aparecen en el encabezado de los informes PDF.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="clinicName"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Nombre comercial</FormLabel>
                    <FormControl>
                      <Input placeholder="Mi Clínica Veterinaria" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="clinicNit"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>NIT</FormLabel>
                    <FormControl>
                      <Input placeholder="900000000-0" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="address"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Dirección</FormLabel>
                    <FormControl>
                      <Input placeholder="Calle 12 # 34-56" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="phone"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Teléfono</FormLabel>
                    <FormControl>
                      <Input placeholder="300 000 0000" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="city"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Ciudad</FormLabel>
                    <FormControl>
                      <Input placeholder="Bogotá D.C." {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="tagline"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Texto de marca</FormLabel>
                    <FormControl>
                      <Input placeholder="Clínica Veterinaria" {...field} />
                    </FormControl>
                    <FormDescription>
                      Aparece en el login, la barra lateral y «Acerca de».
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="groomingDuration"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Duración por defecto de estética (min)</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        min={5}
                        placeholder="60"
                        {...field}
                        onChange={(e) => field.onChange(e.target.value)}
                      />
                    </FormControl>
                    <FormDescription>
                      Se usa cuando el servicio del catálogo no define duración.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="receptionTicketPrefix"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Prefijo del ticket de recepción</FormLabel>
                    <FormControl>
                      <Input placeholder="T" {...field} />
                    </FormControl>
                    <FormDescription>
                      Opcional: prefijo impreso en el turno de recepción.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="sm:col-span-2">
                <FormLabel>Logo de la clínica</FormLabel>
                <div className="mt-2 flex flex-wrap items-center gap-4">
                  <div className="bg-muted/40 flex h-20 w-36 shrink-0 items-center justify-center overflow-hidden rounded-lg border">
                    {logoContentUrl ? (
                      <img
                        src={logoContentUrl}
                        alt="Logo de la clínica"
                        className="max-h-full max-w-full object-contain"
                      />
                    ) : (
                      <span className="text-muted-foreground px-2 text-center text-xs">
                        Sin logo — se usa el de ISALAB
                      </span>
                    )}
                  </div>
                  <div className="flex flex-col gap-2">
                    <div className="flex items-center gap-2">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={pickLogo}
                        disabled={importLogo.isPending}
                      >
                        {importLogo.isPending ? (
                          <Loader2 className="animate-spin" />
                        ) : (
                          <ImageUp className="size-4" />
                        )}
                        Seleccionar logo…
                      </Button>
                      {logoContentUrl && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={clearLogo}
                          disabled={deleteLogo.isPending}
                        >
                          <X className="size-4" />
                          Quitar
                        </Button>
                      )}
                    </div>
                    <p className="text-muted-foreground text-xs">
                      PNG, JPG o WebP. Se copia a la app y aparece en el
                      encabezado de los reportes PDF.
                    </p>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Logos Secundarios */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <ImageUp className="size-4 text-primary" />
                Logos Secundarios
              </CardTitle>
              <CardDescription>
                Puedes añadir otros logos que podrás elegir al momento de generar el reporte PDF.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  {secondaryLogos?.map((logo) => (
                    <div key={logo.id} className="relative group border rounded-lg p-2 flex flex-col items-center gap-2 bg-muted/20">
                      <div className="h-16 w-full flex items-center justify-center">
                        <img
                          src={
                            secondaryLogoContentUrls[logo.id] ??
                            `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="64" height="48"><text x="50%" y="50%" font-size="10" text-anchor="middle" dy=".35em" fill="#999">${logo.name}</text></svg>`
                          }
                          alt={logo.name}
                          className="max-h-full max-w-full object-contain"
                        />
                      </div>
                      <span className="text-xs font-medium text-center truncate w-full">{logo.name}</span>
                      <Button
                        type="button"
                        variant="destructive"
                        size="icon"
                        className="absolute -top-2 -right-2 h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                        onClick={() => removeSecondaryLogo(logo.id)}
                        disabled={deleteSecondaryLogo.isPending}
                      >
                        <Trash2 className="size-3" />
                      </Button>
                    </div>
                  ))}
                  
                  <div className="border border-dashed rounded-lg p-2 flex items-center justify-center bg-muted/10 h-26 hover:bg-muted/30 transition-colors">
                    <Button
                      type="button"
                      variant="ghost"
                      onClick={pickSecondaryLogo}
                      disabled={importSecondaryLogo.isPending}
                      className="w-full h-full flex flex-col items-center gap-2 text-muted-foreground"
                    >
                      {importSecondaryLogo.isPending ? (
                        <Loader2 className="animate-spin size-6" />
                      ) : (
                        <ImageUp className="size-6" />
                      )}
                      <span className="text-xs">Añadir Logo</span>
                    </Button>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Facturación */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <CreditCard className="size-4 text-primary" />
                Facturación
              </CardTitle>
              <CardDescription>
                Valores por defecto para la emisión de facturas.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="taxRate"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>IVA por defecto (%)</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        step="0.01"
                        min={0}
                        max={100}
                        {...field}
                        value={(field.value as number | undefined) ?? ""}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="currency"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Moneda</FormLabel>
                    <FormControl>
                      <Input placeholder="COP" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </CardContent>
          </Card>

          {/* Firma de reportes */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <PenLine className="size-4 text-primary" />
                Firma de reportes
              </CardTitle>
              <CardDescription>
                Médico veterinario que firma los informes PDF.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="signatureMode"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Modo de firma</FormLabel>
                    <Select
                      value={field.value}
                      onValueChange={field.onChange}
                    >
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value="GRAPHIC">
                          Gráfica (texto del firmante)
                        </SelectItem>
                        <SelectItem value="DIGITAL">
                          Digital PKCS#12
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <FormDescription>
                      La firma digital agrega el bloque de firma con metadatos
                      del certificado (Ley 527 de 1999). Requiere un certificado
                      PKCS#12 (.p12/.pfx).
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="vetName"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Nombre del veterinario</FormLabel>
                    <FormControl>
                      <Input placeholder="Dra. Ana Pérez" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="vetLicense"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Tarjeta profesional MVZ</FormLabel>
                    <FormControl>
                      <Input placeholder="Nº de registro" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="pkcs12Password"
                render={({ field }) => (
                  <FormItem className="sm:col-span-2">
                    <FormLabel>Contraseña del certificado (no se guarda)</FormLabel>
                    <FormControl>
                      <Input
                        type="password"
                        placeholder="••••••••"
                        autoComplete="off"
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      Se usa para validar el certificado al importarlo y queda
                      guardada en memoria de la sesión para firmar los reportes.
                      Nunca se persiste en la base de datos: al reiniciar la app
                      deberás reingresarla (Guardar configuración con la
                      contraseña escrita) para volver a firmar.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="space-y-2 sm:col-span-2">
                <Label>Certificado digital (.p12/.pfx)</Label>
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={pickPkcs12}
                    disabled={importPkcs12.isPending}
                  >
                    {importPkcs12.isPending ? (
                      <Loader2 className="animate-spin" />
                    ) : (
                      <FileKey2 className="size-4" />
                    )}
                    {settings?.certificate
                      ? "Reemplazar certificado…"
                      : "Importar certificado…"}
                  </Button>
                  {settings?.certificate && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={clearPkcs12}
                      disabled={deletePkcs12.isPending}
                    >
                      <X className="size-4" />
                      Quitar
                    </Button>
                  )}
                </div>
                {settings?.certificate && (
                  <p className="text-muted-foreground font-mono text-xs break-all">
                    {settings.certificate.fileName ?? settings.certificate.kind}
                    {settings.certificate.sizeBytes > 0
                      ? ` · ${Math.max(1, Math.round(settings.certificate.sizeBytes / 1024))} KB`
                      : ""}
                  </p>
                )}
                <p className="text-muted-foreground text-sm">
                  El certificado se almacena en la base de datos y se usa para
                  firmar los reportes (PAdES, Ley 527 de 1999).
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Inteligencia Artificial */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Bot className="size-4 text-primary" />
                Inteligencia Artificial
              </CardTitle>
              <CardDescription>
                Configura la integración con Groq para la interpretación de resultados de laboratorio.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="groqApiKey"
                render={({ field }) => (
                  <FormItem className="sm:col-span-2">
                    <FormLabel>Groq API Key</FormLabel>
                    <FormControl>
                      <Input type="password" placeholder="gsk_..." {...field} />
                    </FormControl>
                    <FormDescription>
                      Consigue una clave gratuita en <a href="https://console.groq.com" target="_blank" className="text-primary underline">console.groq.com</a>.
                      Debes guardar la configuración antes de probar la conexión.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="sm:col-span-2 flex flex-wrap items-center gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={testGroq}
                  disabled={testingGroq}
                >
                  {testingGroq ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <Wifi className="size-4" />
                  )}
                  Probar conexión
                </Button>
                <p className="text-muted-foreground text-xs">
                  Envía una solicitud mínima a Groq para validar la clave guardada.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Notificaciones por email (SMTP) */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Mail className="size-4 text-primary" />
                Notificaciones por email
              </CardTitle>
              <CardDescription>
                Servidor SMTP para enviar avisos de valores críticos de
                laboratorio al propietario. La contraseña se guarda cifrada.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-2">
              <FormField
                control={form.control}
                name="smtpHost"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Servidor SMTP</FormLabel>
                    <FormControl>
                      <Input placeholder="smtp.gmail.com" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="grid grid-cols-2 gap-3">
                <FormField
                  control={form.control}
                  name="smtpPort"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Puerto</FormLabel>
                      <FormControl>
                        <Input
                          type="number"
                          placeholder="587"
                          {...field}
                          value={(field.value as number | undefined) ?? ""}
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="smtpTls"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Seguridad</FormLabel>
                      <Select value={field.value} onValueChange={field.onChange}>
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          <SelectItem value="STARTTLS">STARTTLS (587)</SelectItem>
                          <SelectItem value="TLS">TLS implícito (465)</SelectItem>
                          <SelectItem value="NONE">Sin cifrado (local)</SelectItem>
                        </SelectContent>
                      </Select>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
              <FormField
                control={form.control}
                name="smtpUsername"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Usuario</FormLabel>
                    <FormControl>
                      <Input placeholder="lab@clinica.com" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="smtpPassword"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Contraseña</FormLabel>
                    <FormControl>
                      <Input
                        type="password"
                        placeholder="••••••••"
                        autoComplete="off"
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      Se guarda cifrada. Si ya existe una contraseña, deja el
                      campo sin tocar para conservarla.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="smtpFrom"
                render={({ field }) => (
                  <FormItem className="sm:col-span-2">
                    <FormLabel>Remitente</FormLabel>
                    <FormControl>
                      <Input
                        placeholder="ISALAB <no-responder@clinica.com>"
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      Nombre y dirección que aparecerán como remitente de los
                      correos. Guarda la configuración antes de probar.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="sm:col-span-2 flex flex-wrap items-center gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={testSmtp}
                  disabled={testingSmtp}
                >
                  {testingSmtp ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <Wifi className="size-4" />
                  )}
                  Probar conexión
                </Button>
                <p className="text-muted-foreground text-xs">
                  Conecta con el servidor guardado (NOOP) para validar la
                  configuración SMTP.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Mantenimiento y Respaldo */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <DatabaseBackup className="size-4 text-primary" />
                Mantenimiento y Respaldo
              </CardTitle>
              <CardDescription>
                Crea una copia de seguridad en un archivo .zip con toda la base de datos y recursos (firmas, logos).
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Button 
                type="button" 
                variant="outline" 
                onClick={createBackup} 
                disabled={backingUp}
                className="gap-2"
              >
                {backingUp ? <Loader2 className="animate-spin size-4" /> : <DatabaseBackup className="size-4" />}
                Descargar Copia de Seguridad
              </Button>
            </CardContent>
          </Card>

          <Separator />

          <div className="flex justify-end">
            <Button type="submit" disabled={save.isPending}>
              {save.isPending ? (
                <Loader2 className="animate-spin" />
              ) : (
                <Save className="size-4" />
              )}
              Guardar configuración
            </Button>
          </div>
        </form>
      </Form>

      {/* Equipos de laboratorio: gestión de equipos y rangos por marca/modelo
          (fuera del form principal porque tiene sus propios formularios). */}
      <AnalyzerManagementCard />
    </div>
  );
}
