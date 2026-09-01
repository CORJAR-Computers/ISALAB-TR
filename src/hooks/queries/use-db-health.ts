import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function useDbHealth() {
  return useQuery({
    queryKey: ["db-health"],
    queryFn: api.dbHealth,
    retry: false,
  });
}
