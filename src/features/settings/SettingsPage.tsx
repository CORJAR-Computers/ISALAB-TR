import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Building2, CreditCard, ImageUp, Loader2, PenLine, Save, X } from "lucide-react";
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
} from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import type { ClinicSettings } from "@/bindings";

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
});

type Values = z.infer<typeof schema>;

const toNullable = (v: unknown) =>
  typeof v === "string" && v.trim() !== "" ? v.trim() : null;

export function SettingsPage() {
  const { data: settings, isLoading } = useClinicSettings();
  const save = useSaveClinicSettings();
  const importLogo = useImportClinicLogo();

  const form = useForm<Values>({
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
    };
    try {
      await save.mutateAsync(input);
      toast.success("Configuración guardada", {
        description:
          "Los reportes PDF y la facturación usarán estos datos.",
      });
    } catch (e) {
      toast.error("No se pudo guardar", { description: getErrorMessage(e) });
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
                          Digital PKCS#12 (próximamente)
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <FormDescription>
                      La firma digital aún no está implementada; usa Gráfica.
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
    </div>
  );
}
