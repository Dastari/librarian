/**
 * Where the remote's on-screen keyboard goes (a Glass UX component, still to come): a
 * Netflix-style letter grid in the search page's left column, sized for its keys so the
 * layout does not move when it arrives. Until then the space is marked and the search field
 * above takes text from the TV's own keyboard or a connected one.
 */
export function SearchKeyboardSlot() {
  return (
    <div data-osk-slot aria-hidden className="grid min-h-[20rem] place-items-center rounded-card border border-dashed border-separator p-6 text-center">
      <p className="text-label-sm text-muted">On-screen keyboard</p>
    </div>
  );
}
