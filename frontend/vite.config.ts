import { defineConfig, loadEnv } from 'vite'
import { devtools } from '@tanstack/devtools-vite'
import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

import { tanstackRouter } from '@tanstack/router-plugin/vite'
import { fileURLToPath, URL } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  // This address is used by the Vite server, never by the visitor's browser.
  const backendTarget = env.BACKEND_PROXY_TARGET || 'http://127.0.0.1:3001'
  const publicDevUrl = env.DEV_SERVER_PUBLIC_URL
    ? new URL(env.DEV_SERVER_PUBLIC_URL)
    : undefined

  return {
    server: {
      host: '0.0.0.0',
      port: 3000,
      strictPort: true,
      allowedHosts: ['librarian.dastari.net', ...(publicDevUrl ? [publicDevUrl.hostname] : [])],
      ws: publicDevUrl ? {
        protocol: publicDevUrl.protocol === 'https:' ? 'wss' : 'ws',
        host: publicDevUrl.hostname,
        clientPort: Number(publicDevUrl.port || (publicDevUrl.protocol === 'https:' ? 443 : 80)),
      } : undefined,
      proxy: {
        '/api': {
          target: backendTarget,
          changeOrigin: true,
        },
        '/graphql': {
          target: backendTarget,
          changeOrigin: true,
          ws: true,
        },
      },
    },
  plugins: [
    mode === 'development' && devtools({
      // Upstream console piping hard-codes localhost in its editor links.
      // Keep browser logs native when accessing Vite through a public origin.
      consolePiping: { enabled: !publicDevUrl },
      enhancedLogs: { enabled: !publicDevUrl },
    }),
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
    }),
    viteReact(),
    tailwindcss(),
  ].filter(Boolean),
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  build: {
    chunkSizeWarningLimit: 1000, // HeroUI + React is large
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          if (!id.includes('node_modules/')) return
          
          // GraphQL/Apollo - independent data layer
          if (id.includes('/graphql') || id.includes('@apollo/')) {
            return 'vendor-graphql'
          }
          // Everything else (React, HeroUI, Router, etc.) goes to vendor
          // This avoids circular dependency issues
          return 'vendor'
        },
      },
    },
  },
  }
})
