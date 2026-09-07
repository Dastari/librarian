// @vitest-environment node
import { describe, expect, it } from "vitest";

import { imageUrl } from "../graph";

describe("constellation image URLs", () => {
  it("expands the packed `wiki:hash:width:file` form the generator writes", () => {
    expect(imageUrl("en:9/95:250:Thor_%28film%29_poster.jpg")).toBe(
      "https://upload.wikimedia.org/wikipedia/en/thumb/9/95/Thor_%28film%29_poster.jpg/250px-Thor_%28film%29_poster.jpg",
    );
    expect(imageUrl("commons:7/7b:250:Stan_Lee.jpg")).toBe("https://upload.wikimedia.org/wikipedia/commons/thumb/7/7b/Stan_Lee.jpg/250px-Stan_Lee.jpg");
  });

  it("keeps an unusual path verbatim", () => {
    expect(imageUrl("en/8/86/The_Silence_of_the_Lambs_poster.jpg")).toBe("https://upload.wikimedia.org/wikipedia/en/8/86/The_Silence_of_the_Lambs_poster.jpg");
  });

  it("returns null when the page had no image", () => {
    expect(imageUrl("")).toBeNull();
  });
});
