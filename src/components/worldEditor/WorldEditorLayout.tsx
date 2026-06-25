interface WorldEditorLayoutProps {
  topBar: React.ReactNode;
  sidebar: React.ReactNode;
  /** null = hidden (Metadata spans full content width) */
  listPanel: React.ReactNode | null;
  formPanel: React.ReactNode;
}

export function WorldEditorLayout({
  topBar,
  sidebar,
  listPanel,
  formPanel,
}: WorldEditorLayoutProps) {
  return (
    <div className="flex flex-col h-screen bg-gray-50 dark:bg-navy-900 overflow-hidden">
      {topBar}

      <div className="flex flex-1 overflow-hidden">
        {/* Col 1: sidebar nav */}
        <div className="w-52 flex-shrink-0 border-r border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800 overflow-hidden flex flex-col">
          {sidebar}
        </div>

        {/* Col 2: entity list (hidden for Metadata) */}
        {listPanel !== null && (
          <div className="w-72 flex-shrink-0 border-r border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-800 overflow-hidden flex flex-col">
            {listPanel}
          </div>
        )}

        {/* Col 3: form panel (spans full width when list is hidden) */}
        <div className="flex-1 overflow-y-auto scrollbar-thin p-6">
          {formPanel}
        </div>
      </div>
    </div>
  );
}
