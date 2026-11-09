import { useEffect, useRef } from "react";

/** Elements that take typing themselves: a key pressed in one stays there. */
function takesText(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  if (target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
  if (target instanceof HTMLInputElement) return !["button", "checkbox", "radio", "range", "submit", "reset", "color", "file"].includes(target.type);
  return false;
}

/**
 * On the search page, typing goes into the search whatever has focus: a result, a tab, a key of
 * the on-screen keyboard. Letters, digits and space are added, Backspace takes one off. Keys with
 * Ctrl, Cmd or Alt are left alone so app shortcuts still reach their targets, and so is typing in
 * a field or inside an open dialog.
 */
export function useTypeToSearch(onType: (update: (query: string) => string) => void, focusInput?: () => void) {
  const handlers = useRef({ onType, focusInput });
  handlers.current = { onType, focusInput };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing || event.ctrlKey || event.metaKey || event.altKey) return;
      const printable = event.key.length === 1;
      if (!printable && event.key !== "Backspace") return;
      if (takesText(event.target)) return;
      if (event.target instanceof Element && event.target.closest("[role=dialog], [role=alertdialog], [role=menu], [role=listbox]")) return;
      event.preventDefault();
      event.stopPropagation();
      const { onType: type, focusInput: focus } = handlers.current;
      type((query) => (printable ? query + event.key : query.slice(0, -1)));
      focus?.();
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, []);
}
