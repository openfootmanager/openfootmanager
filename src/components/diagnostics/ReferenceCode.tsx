import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy } from "lucide-react";

import { logError } from "../../lib/logger";

interface ReferenceCodeProps {
  code: string;
  /** `lg` where the code is the point of the screen; `md` in a list. */
  size?: "md" | "lg";
}

/**
 * A bug report's reference code, with a way to copy it that does not need a mouse.
 *
 * The code exists to be quoted to the maintainer. `select-all` makes one click take the whole of
 * it, but a `<code>` cannot take focus, so without the button a keyboard or screen-reader user
 * had no way to copy the one thing the screen asks them to keep.
 */
export function ReferenceCode({ code, size = "md" }: ReferenceCodeProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
    } catch (error: unknown) {
      logError(`[report] could not copy the reference code: ${String(error)}`);
      setCopied(false);
    }
  };

  return (
    <span className="inline-flex items-center gap-2">
      <code
        className={`select-all font-mono tracking-widest text-gray-900 dark:text-gray-100 ${
          size === "lg" ? "text-xl" : "text-sm"
        }`}
      >
        {code}
      </code>
      <button
        type="button"
        onClick={() => {
          void copy();
        }}
        aria-label={t("reportBug.copyCode")}
        title={t("reportBug.copyCode")}
        className="p-1 rounded-md text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-600 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 dark:focus:ring-offset-navy-800"
      >
        <Copy className="w-4 h-4" aria-hidden="true" />
      </button>
      {/* Always rendered, so the announcement is a change to a live region that already exists. */}
      <span role="status" className="text-xs text-primary-700 dark:text-primary-400">
        {copied ? t("reportBug.codeCopied") : ""}
      </span>
    </span>
  );
}
