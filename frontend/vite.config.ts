import { defineConfig } from 'vite'
import { devtools } from '@tanstack/devtools-vite'
import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

import { tanstackRouter } from '@tanstack/router-plugin/vite'
import { fileURLToPath, URL } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig({
  server: {
    host: '0.0.0.0',
    port: 3000,
  },
  plugins: [
    devtools(),
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
    }),
    viteReact(),
    tailwindcss(),
  ],
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
          
          // Supabase - independent, used for auth
          if (id.includes('@supabase/')) {
            return 'vendor-supabase'
          }
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
})
