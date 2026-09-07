/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/react" />

declare module "*.svg" {
  const src: string;
  export default src;
}

declare const __APP_VERSION__: string;
declare const __BUILD_TIME__: string;
