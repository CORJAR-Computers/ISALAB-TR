import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function useDashboardStats() {
  return useQuery({
    queryKey: ["dashboard"],
    queryFn: api.getDashboardStats,
    refetchInterval: 30_000,
  });
}
