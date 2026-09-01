import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { ClinicSettings } from "@/bindings";

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
    onSuccess: () => qc.invalidateQueries({ queryKey: ["clinic-settings"] }),
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
