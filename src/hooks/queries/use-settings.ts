import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { ClinicSettings } from "@/bindings";

/**
 * Marca pública de la clínica (nombre comercial + tagline).
 * No exige sesión: el login la usa para mostrar el texto de marca.
 */
export function useClinicBranding() {
  return useQuery({
    queryKey: ["clinic-branding"],
    queryFn: api.getClinicBranding,
    staleTime: 5 * 60 * 1000,
  });
}

export function useClinicSettings() {
  return useQuery({
    queryKey: ["clinic-settings"],
    queryFn: api.getClinicSettings,
  });
}

export function useSaveClinicSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: ClinicSettings) => api.saveClinicSettings(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["clinic-settings"] });
      qc.invalidateQueries({ queryKey: ["clinic-branding"] });
    },
  });
}

export function useImportClinicLogo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (sourcePath: string) => api.importClinicLogo(sourcePath),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["clinic-settings"] }),
  });
}

export function useDeleteClinicLogo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.deleteClinicLogo,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["clinic-settings"] }),
  });
}

export function useImportPkcs12() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ sourcePath, password }: { sourcePath: string; password: string }) =>
      api.importPkcs12(sourcePath, password),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["clinic-settings"] }),
  });
}

export function useDeletePkcs12() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.deletePkcs12,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["clinic-settings"] }),
  });
}

export function useSecondaryLogos() {
  return useQuery({
    queryKey: ["secondary-logos"],
    queryFn: api.listSecondaryLogos,
  });
}

export function useImportSecondaryLogo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ name, sourcePath }: { name: string; sourcePath: string }) =>
      api.importSecondaryLogo(name, sourcePath),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secondary-logos"] }),
  });
}

export function useDeleteSecondaryLogo() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteSecondaryLogo(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secondary-logos"] }),
  });
}
