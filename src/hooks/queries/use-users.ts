import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { ChangePasswordInput, CreateUserInput } from "@/bindings";
import { useSessionStore } from "@/stores/session-store";

export function useUsers() {
  return useQuery({ queryKey: ["users"], queryFn: api.listUsers });
}

export function useCreateUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateUserInput) => api.createUser(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["users"] }),
  });
}

/** Cambia la contraseña del usuario con sesión activa y actualiza el store. */
export function useChangePassword() {
  const setSession = useSessionStore((s) => s.setSession);
  return useMutation({
    mutationFn: (input: ChangePasswordInput) => api.changePassword(input),
    onSuccess: (user) => setSession(user),
  });
}
