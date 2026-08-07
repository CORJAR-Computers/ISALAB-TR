import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Building2, CreditCard, FileKey2, ImageUp, Loader2, PenLine, Save, X, Bot, DatabaseBackup, Wifi } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
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
  useImportClinicLogo,
  useSaveClinicSettings,
  useSecondaryLogos,
  useImportSecondaryLogo,
  useDeleteSecondaryLogo,
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
  logoPath: z.string().optional(),
  taxRate: z.coerce.number().min(0).max(100, "IVA entre 0 y 100"),
  currency: z.string().min(1, "Moneda obligatoria"),
  signatureMode: z.enum(["GRAPHIC", "DIGITAL"]),
  vetName: z.string().optional(),
  vetLicense: z.string().optional(),
  groqApiKey: z.string().optional(),
  pkcs12Path: z.string().optional(),
  pkcs12Password: z.string().optional(),
});

type Values = z.infer<typeof schema>;

const toNullable = (v: unknown) =>
  typeof v === "string" && v.trim() !== "" ? v.trim() : null;

export function SettingsPage() {
  const { data: settings, isLoading } = useClinicSettings();
  const save = useSaveClinicSettings();
  const importLogo = useImportClinicLogo();
  const { data: secondaryLogos } = useSecondaryLogos();
  const importSecondaryLogo = useImportSecondaryLogo();
  const deleteSecondaryLogo = useDeleteSecondaryLogo();

  const [backingUp, setBackingUp] = useState(false);
  const [testingGroq, setTestingGroq] = useState(false);
  const [importingPkcs12, setImportingPkcs12] = useState(false);

  const form = useForm<z.input<typeof schema>, unknown, z.output<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: {
      clinicName: "",
      clinicNit: "",
      address: "",
      phone: "",
      city: "",
      logoPath: "",
      taxRate: 19,
      currency: "COP",
      signatureMode: "GRAPHIC",
      vetName: "",
      vetLicense: "",
      groqApiKey: "",
      pkcs12Path: "",
      pkcs12Password: "",
    },
  });

  // Carga los valores guardados cuando llegan del backend.
  useEffect(() => {
    if (settings) {
      form.reset({
        clinicName: settings.clinicName,
        clinicNit: settings.clinicNit,
        address: settings.address ?? "",
        phone: settings.phone ?? "",
        city: settings.city ?? "",
        logoPath: settings.logoPath ?? "",
        taxRate: settings.taxRate ?? 19,
        currency: settings.currency,
        signatureMode:
          settings.signatureMode === "DIGITAL" ? "DIGITAL" : "GRAPHIC",
        vetName: settings.vetName ?? "",
        vetLicense: settings.vetLicense ?? "",
        groqApiKey: settings.groqApiKey ?? "",
        pkcs12Path: settings.pkcs12Path ?? "",
        pkcs12Password: "",
      });
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
      const stored = await importLogo.mutateAsync(selected);
      form.setValue("logoPath", stored, { shouldDirty: true });
      toast.success("Logo seleccionado", {
        description: "Guarda la configuración para aplicarlo a los reportes.",
      });
    } catch (e) {
      toast.error("No se pudo cargar el logo", {
        description: getErrorMessage(e),
      });
    }
  };

  const clearLogo = () => {
    form.setValue("logoPath", "", { shouldDirty: true });
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

  const logoPreview = form.watch("logoPath");

  const onSubmit = async (values: Values) => {
    const input: ClinicSettings = {
      clinicName: values.clinicName.trim(),
      clinicNit: values.clinicNit.trim(),
      address: toNullable(values.address),
      phone: toNullable(values.phone),
      city: toNullable(values.city),
      logoPath: toNullable(values.logoPath),
      taxRate: values.taxRate,
      currency: values.currency.trim(),
      signatureMode: values.signatureMode,
      vetName: values.vetName?.trim() ?? "",
      vetLicense: toNullable(values.vetLicense),
      groqApiKey: toNullable(values.groqApiKey),
      pkcs12Path: toNullable(values.pkcs12Path),
      // La contraseña solo viaja al backend para validarla y guardarla en
      // memoria; nunca se persiste. Se limpia tras guardar.
      pkcs12Password: toNullable(values.pkcs12Password),
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

      setImportingPkcs12(true);
      const password = form.getValues("pkcs12Password") ?? "";
      if (!password) {
        toast.error("Falta la contraseña del certificado", {
          description: "Ingresa la contraseña del .p12 antes de importarlo.",
        });
        setImportingPkcs12(false);
        return;
      }
      const stored = await api.importPkcs12(selected, password);
      form.setValue("pkcs12Path", stored, { shouldDirty: true });
      toast.success("Certificado importado", {
        description: "Guarda la configuración para aplicar la firma digital.",
      });
    } catch (e) {
      toast.error("No se pudo importar el certificado", {
        description: getErrorMessage(e),
      });
    } finally {
      setImportingPkcs12(false);
    }
  };

  const clearPkcs12 = () => {
    form.setValue("pkcs12Path", "", { shouldDirty: true });
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

  const createBackup = async () => {
    try {
      const selected = await saveDialog({
        title: "Crear copia de seguridad",
        filters: [{ name: "Archivos ZIP", extensions: ["zip"] }],
        defaultPath: `ISALAB_Backup_${new Date().toISOString().split("T")[0]}.zip`,
      });
      if (!selected) return;

      setBackingUp(true);
      const destPath = await invoke<string>("create_local_backup", { destPath: selected });
      toast.success("Copia de seguridad creada con éxito", {
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
              <div className="sm:col-span-2">
                <FormLabel>Logo de la clínica</FormLabel>
                <div className="mt-2 flex flex-wrap items-center gap-4">
                  <div className="bg-muted/40 flex h-20 w-36 shrink-0 items-center justify-center overflow-hidden rounded-lg border">
                    {logoPreview ? (
                      <img
                        src={convertFileSrc(logoPreview)}
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
                      {logoPreview && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={clearLogo}
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
                          src={convertFileSrc(logo.logoPath)}
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
              <FormField
                control={form.control}
                name="pkcs12Path"
                render={({ field }) => (
                  <FormItem className="sm:col-span-2">
                    <FormLabel>Certificado digital (.p12/.pfx)</FormLabel>
                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={pickPkcs12}
                        disabled={importingPkcs12}
                      >
                        {importingPkcs12 ? (
                          <Loader2 className="animate-spin" />
                        ) : (
                          <FileKey2 className="size-4" />
                        )}
                        {field.value ? "Reemplazar certificado…" : "Importar certificado…"}
                      </Button>
                      {field.value && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={clearPkcs12}
                        >
                          <X className="size-4" />
                          Quitar
                        </Button>
                      )}
                    </div>
                    {field.value && (
                      <p className="text-muted-foreground font-mono text-xs break-all">
                        {field.value}
                      </p>
                    )}
                    <FormDescription>
                      El archivo se copia a la carpeta de la app; la ruta se
                      guarda en la configuración. Los reportes se firman
                      criptográficamente (PAdES, Ley 527 de 1999) con este
                      certificado.
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
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
