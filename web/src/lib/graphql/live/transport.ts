/** Kept independent of Apollo construction to avoid provider/client cycles. */
const listeners = new Set<(connected: boolean) => void>();
export const liveTransport = {
  emit(connected: boolean) { for (const listener of listeners) listener(connected); },
  subscribe(listener: (connected: boolean) => void) { listeners.add(listener); return () => { listeners.delete(listener); }; },
};
