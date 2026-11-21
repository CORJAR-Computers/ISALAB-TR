import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'node:path'
import { readFileSync } from 'node:fs'

const pkg = JSON.parse(
  readFileSync(new URL('./package.json', import.meta.url), 'utf-8'),
) as { version: string }

export default defineConfig({
  plugins: [react()],
  define: {
    // Mismo define que vite.config.ts para que los tests resuelvan __APP_VERSION__.
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    css: true,
    coverage: {
      provider: 'v8',
      // La cobertura se mide solo donde hoy hay red: los módulos con tests y
      // las reglas de dinero que comparten pantallas. El umbral es un suelo
      // (baja solo con un cambio explícito), no una meta: incluir todo src/
      // daría 0 % en pantallas sin tests y el gate sería ruido.
      include: [
        'src/features/cash/money.ts',
        // Pantallas del dominio de dinero y sesión: ahora tienen tests de
        // componente, así que entran en el gate con su propio suelo.
        'src/features/cash/CashPage.tsx',
        'src/features/invoices/InvoicesPage.tsx',
        'src/features/auth/LoginPage.tsx',
        'src/lib/status.ts',
        'src/lib/utils.ts',
        'src/lib/whatsapp.ts',
      ],
      reporter: ['text-summary', 'text', 'json-summary'],
      // Umbral por módulo (no global): una pantalla nueva sin tests no diluye
      // el suelo de las reglas de dinero, y bajar un umbral exige tocar esta
      // lista de forma explícita.
      thresholds: {
        'src/features/cash/money.ts': {
          lines: 95,
          statements: 85,
          functions: 95,
          branches: 70,
        },
        'src/lib/*.ts': {
          lines: 95,
          statements: 85,
          functions: 95,
          branches: 70,
        },
        // Suelos medidos con la suite actual, no aspiracionales: cubren el
        // resumen de caja, el historial con diferencia y el formulario de
        // apertura; suben cuando se añadan tests, nunca bajan sin motivo.
        'src/features/cash/CashPage.tsx': {
          lines: 75,
          statements: 75,
          functions: 60,
          branches: 60,
        },
        'src/features/invoices/InvoicesPage.tsx': {
          lines: 60,
          statements: 60,
          functions: 40,
          branches: 60,
        },
        'src/features/auth/LoginPage.tsx': {
          lines: 80,
          statements: 80,
          functions: 60,
          branches: 55,
        },
      },
    },
  },
})
