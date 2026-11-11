interface EntityListPanelProps {
  children: React.ReactNode;
}

export function EntityListPanel({ children }: EntityListPanelProps) {
  return (
    <div data-scroll-container="" className="h-full overflow-y-auto scrollbar-thin p-3">
      {children}
    </div>
  );
}
