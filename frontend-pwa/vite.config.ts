/**
 * Configure Vite for the repository-owned frontend PWA.
 *
 * This module wires the React and Tailwind plugins and pins the dev and
 * preview server contract to the repository's expected host and port.
 */
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig, loadEnv } from 'vite';

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  const proxyTarget = env.CORBUSIER_API_PROXY_TARGET;
  const bearerToken = env.CORBUSIER_DEV_BEARER_TOKEN;

  return {
    plugins: [react(), tailwindcss()],
    server: {
      host: '127.0.0.1',
      port: 4173,
      proxy: proxyTarget
        ? {
            '/api': {
              changeOrigin: true,
              headers: bearerToken
                ? { Authorization: `Bearer ${bearerToken}` }
                : undefined,
              secure: false,
              target: proxyTarget,
            },
          }
        : undefined,
      strictPort: true,
    },
    preview: {
      host: '127.0.0.1',
      port: 4173,
      strictPort: true,
    },
  };
});
