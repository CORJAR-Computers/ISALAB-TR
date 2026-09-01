import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

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
  return useQuery({ queryKey: ["analytes"], queryFn: api.listAnalytes });
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
