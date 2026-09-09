import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api } from "@/lib/api";
import { getErrorMessage } from "@/lib/api";
import type { Analyte, CreateAnalyteInput } from "@/bindings";

export function useSpecies() {
  return useQuery({ queryKey: ["species"], queryFn: api.listSpecies });
}

export function useBreeds(speciesId: number | null) {
  return useQuery({
    queryKey: ["breeds", speciesId],
    queryFn: () => api.listBreeds(speciesId!),
    enabled: speciesId != null,
  });
}

export function useSampleTypes() {
  return useQuery({
    queryKey: ["sample-types"],
    queryFn: api.listSampleTypes,
  });
}

export function useAnalytes() {
  return useQuery({
    queryKey: ["analytes"],
    queryFn: api.listAnalytes,
    // Mantiene los ítems montados durante el refetch tras crear un analito:
    // si `data` pasa por undefined, los <SelectItem> se desmontan y Radix
    // Select pierde el valor seleccionado (lo resetea a "").
    placeholderData: (prev) => prev,
  });
}

export function useCreateAnalyte() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateAnalyteInput) => api.createAnalyte(input),
    onSuccess: (analyte) => {
      // Optimista: el catálogo ya incluye el analito nuevo cuando el diálogo
      // lo selecciona. Sin esto, si el valor del <Select> se fija antes de
      // que el ítem esté montado, el bubble input de Radix Select (un <select>
      // nativo oculto) no encuentra la option y emite onValueChange(""),
      // pisando la selección recién hecha con 0.
      queryClient.setQueryData<Analyte[]>(["analytes"], (old) =>
        old ? [...old, analyte] : [analyte],
      );
      queryClient.invalidateQueries({ queryKey: ["analytes"] });
      toast.success("Analito creado", {
        description: `${analyte.name} ya está disponible en el catálogo.`,
      });
    },
    onError: (e) => toast.error("No se pudo crear el analito", { description: getErrorMessage(e) }),
  });
}

export function useVaccineTypes() {
  return useQuery({
    queryKey: ["vaccine-types"],
    queryFn: api.listVaccineTypes,
  });
}

export function useOwners(search: string) {
  return useQuery({
    queryKey: ["owners", search],
    queryFn: () => api.listOwners(search.trim() || null),
    placeholderData: (prev) => prev,
  });
}
