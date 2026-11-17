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
  return useMutation({
    mutationFn: (sourcePath: string) => api.importClinicLogo(sourcePath),
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
