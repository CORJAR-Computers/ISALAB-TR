import { useMutation } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { LoginInput } from "@/bindings";
import { useSessionStore } from "@/stores/session-store";
import { useUiStore } from "@/stores/ui-store";

export function useLogin() {
  const setSession = useSessionStore((s) => s.setSession);
  return useMutation({
    mutationFn: (input: LoginInput) => api.login(input),
    onSuccess: (user) => {
      setSession(user);
      // Tras autenticarse siempre se abre el Panel de control.
      useUiStore.getState().navigate("dashboard");
    },
  });
}

export function useLogout() {
  const setSession = useSessionStore((s) => s.setSession);
  return useMutation({
    mutationFn: () => api.logout(),
    onSuccess: () => setSession(null),
  });
}
