import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function useAuditLog(
  page: number,
  pageSize = 50,
  username?: string,
  action?: string,
  dateFrom?: string,
  dateTo?: string,
) {
  const offset = page * pageSize;
  return useQuery({
    queryKey: ["audit-log", page, pageSize, username, action, dateFrom, dateTo],
    queryFn: () => api.listAuditLog(pageSize, offset, username, action, dateFrom, dateTo),
    placeholderData: (prev) => prev,
  });
}
