import { ArrowLeft, FileArchive, FolderOpen, Plus } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";

import type { SamplePackage } from "../../menu/PackageEditor/sampleData";
import { Button } from "../../ui";

interface WorldsSidebarProps {
  busy: boolean;
  samples: readonly SamplePackage[];
  onNewWorld: () => void;
  onOpenFile: () => void;
  onOpenFolder: () => void;
  onStartFromSample: (sample: SamplePackage) => void;
  onBack: () => void;
}

/**
 * The World Editor's way in, on the always-dark sidebar: a new world, an .ofm file or a folder to open,
 * the examples to start from, and the way back to the main menu. It holds no list of worlds: those are
 * the table's.
 */
export default function WorldsSidebar({
  busy,
  samples,
  onNewWorld,
  onOpenFile,
  onOpenFolder,
  onStartFromSample,
  onBack,
}: WorldsSidebarProps) {
  const { t } = useTranslation();
  const examplesId = useId();
  return (
    <aside
      aria-label={t("worldEditor.title")}
      className="dark flex w-full shrink-0 flex-col gap-5 border-b border-line bg-chrome p-5 text-fg md:sticky md:top-0 md:h-screen md:w-64 md:overflow-hidden md:border-b-0 md:border-r"
    >
      <div className="flex flex-col gap-1">
        <p className="font-heading text-title font-bold uppercase leading-none tracking-label">
          {t("worldEditor.title")}
        </p>
        <p className="text-caption text-fg-muted">{t("worldEditor.home.subtitle")}</p>
      </div>

      <div className="flex flex-col gap-2">
        <Button
          className="w-full"
          disabled={busy}
          onClick={onNewWorld}
          icon={<Plus aria-hidden="true" />}
        >
          {t("worldEditor.home.newWorld")}
        </Button>
        <Button
          variant="outline"
          className="w-full"
          disabled={busy}
          onClick={onOpenFile}
          icon={<FileArchive aria-hidden="true" />}
        >
          {t("worldEditor.home.openFile")}
        </Button>
        <Button
          variant="outline"
          className="w-full"
          disabled={busy}
          onClick={onOpenFolder}
          icon={<FolderOpen aria-hidden="true" />}
        >
          {t("worldEditor.home.openFolder")}
        </Button>
      </div>

      <section
        aria-labelledby={examplesId}
        className="flex flex-col gap-1 md:min-h-0 md:flex-1 md:overflow-y-auto"
      >
        <p
          id={examplesId}
          className="px-2.5 text-caption font-bold uppercase tracking-label text-fg-subtle"
        >
          {t("worldEditor.home.examples")}
        </p>
        <ul className="flex flex-col gap-0.5">
          {samples.map((sample) => (
            <li key={sample.meta.id}>
              <button
                type="button"
                disabled={busy}
                onClick={() => onStartFromSample(sample)}
                className="flex w-full flex-col gap-0.5 rounded-lg px-2.5 py-2 text-left hover:bg-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus disabled:cursor-not-allowed disabled:opacity-50"
              >
                <span className="text-body font-semibold">{sample.meta.name}</span>
                <span className="text-caption text-fg-subtle">{sample.meta.description}</span>
              </button>
            </li>
          ))}
        </ul>
      </section>

      <button
        type="button"
        onClick={onBack}
        data-keep-in-window
        className="mt-auto inline-flex shrink-0 items-center gap-2 rounded-lg px-2.5 py-1 text-body text-fg-muted hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        <ArrowLeft aria-hidden="true" className="size-4" />
        {t("worldEditor.home.backToMenu")}
      </button>
    </aside>
  );
}
