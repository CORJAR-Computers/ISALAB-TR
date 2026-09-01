import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { CreateVaccineInput } from "@/bindings";

export function useVaccines(search: string) {
  return useQuery({
    queryKey: ["vaccines", search],
    queryFn: () => api.listVaccines(search.trim() || null),
    placeholderData: (prev) => prev,
  });
}

export function useCreateVaccine() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateVaccineInput) => api.createVaccine(input),
    onSuccess: (v) => {
      qc.invalidateQueries({ queryKey: ["vaccines"] });
      qc.invalidateQueries({ queryKey: ["clinical-history", v.patientId] });
      qc.invalidateQueries({ queryKey: ["dashboard"] });
    },
  });
}
