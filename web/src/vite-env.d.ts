/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/vanillajs" />

declare module "*.svg" {
  const src: string;
  export default src;
}

declare const __APP_VERSION__: string;
declare const __BUILD_TIME__: string;
/** Unique per production build; `version.json` carries the deployed one. */
declare const __BUILD_ID__: string;
