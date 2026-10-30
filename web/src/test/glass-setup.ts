import { act } from "@testing-library/react";
import { toast } from "glass-ux/toast";
import { afterEach } from "vitest";

if (typeof document !== "undefined") {
  // Vitest omits stylesheets. Mirror the package's visibility rule so inactive
  // status faces reserve space without joining a button's accessible name.
  const style = document.createElement("style");
  style.textContent = ".fa-btn-face:not([data-active]) { visibility: hidden; }";
  document.head.append(style);
}

afterEach(() => {
  // The public manager survives React unmounts; dismiss queued notifications too.
  act(() => toast.dismiss());
});
