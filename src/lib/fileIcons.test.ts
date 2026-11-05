import { expect, test } from "bun:test";
import { FILE_ICON_COLORS, fileIcon } from "./fileIcons";

test("files get the icon of their kind", () => {
  expect(fileIcon("App.svelte").icon).toBe("code");
  expect(fileIcon("data.json").icon).toBe("braces");
  expect(fileIcon("package.json").icon).toBe("package");
  expect(fileIcon("Cargo.lock").icon).toBe("file-lock");
  expect(fileIcon("README.md").icon).toBe("book");
  expect(fileIcon("LICENSE").icon).toBe("scale");
  expect(fileIcon(".env.local").icon).toBe("file-key");
  expect(fileIcon("tsconfig.json").icon).toBe("file-cog");
  expect(fileIcon("vite.config.ts").icon).toBe("file-cog");
  expect(fileIcon(".prettierrc").icon).toBe("file-cog");
  expect(fileIcon("logo.PNG").icon).toBe("image");
  expect(fileIcon("Dockerfile").icon).toBe("container");
  expect(fileIcon("notes").icon).toBe("file");
  expect(fileIcon("app.ts").color).not.toBe(fileIcon("app.rs").color);
});

/** WCAG contrast of a #rrggbb colour against another. */
function contrast(a: string, b: string): number {
  const lum = (hex: string) => {
    const [r, g, bl] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
    return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
  };
  const [x, y] = [lum(a), lum(b)].sort((m, n) => n - m);
  return (x + 0.05) / (y + 0.05);
}

test("every colour stays readable on the panels and on a hovered row", () => {
  for (const surface of ["#1d1a15", "#25211b", "#15130f"]) {
    for (const color of FILE_ICON_COLORS) expect({ color, surface, ratio: contrast(color, surface) >= 3 }).toEqual({ color, surface, ratio: true });
  }
});
