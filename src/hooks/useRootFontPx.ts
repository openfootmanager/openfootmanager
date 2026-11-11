import { useEffect, useState } from "react";

const DEFAULT_ROOT_PX = 16;

const readRootFontPx = () =>
  Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || DEFAULT_ROOT_PX;

/**
 * The root font size in px, kept current when the UI scale changes it (the app sets it on `<html>`).
 * A row height given in rem is `rem × this`, so it grows with the text inside it.
 */
export function useRootFontPx(): number {
  const [px, setPx] = useState(readRootFontPx);

  useEffect(() => {
    const update = () => setPx(readRootFontPx());
    update();
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["style", "class"],
    });
    return () => observer.disconnect();
  }, []);

  return px;
}
