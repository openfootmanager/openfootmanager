import {
  cloneElement,
  isValidElement,
  useEffect,
  useRef,
  useState,
  useCallback,
} from "react";
import { createPortal } from "react-dom";

export interface ContextMenuItem {
  label: string;
  icon?: React.ReactNode;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  divider?: boolean;
}

interface ContextMenuProps {
  items: ContextMenuItem[];
  children: React.ReactNode;
}

export default function ContextMenu({ items, children }: ContextMenuProps) {
  const [visible, setVisible] = useState(false);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const menuRef = useRef<HTMLDivElement>(null);

  const instanceId = useRef(Math.random().toString(36));

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    // Close all other context menus first
    window.dispatchEvent(
      new CustomEvent("close-context-menus", { detail: instanceId.current }),
    );
    const x = Math.min(e.clientX, window.innerWidth - 200);
    const y = Math.min(e.clientY, window.innerHeight - 300);
    setPos({ x, y });
    setVisible(true);
  }, []);

  useEffect(() => {
    const closeFromOther = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail !== instanceId.current) setVisible(false);
    };
    window.addEventListener("close-context-menus", closeFromOther);
    return () =>
      window.removeEventListener("close-context-menus", closeFromOther);
  }, []);

  useEffect(() => {
    if (!visible) return;
    const close = () => setVisible(false);
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [visible]);

  const trigger = isValidElement(children) ? (
    cloneElement(children, {
      onContextMenu: handleContextMenu,
    } as React.HTMLAttributes<HTMLElement>)
  ) : (
    <div onContextMenu={handleContextMenu} className="contents">
      {children}
    </div>
  );

  return (
    <>
      {trigger}
      {visible &&
        createPortal(
          <div
            ref={menuRef}
            className="fixed z-50 min-w-[180px] bg-white dark:bg-navy-800 rounded-lg shadow-xl border border-gray-200 dark:border-navy-600 py-1 animate-in fade-in duration-100"
            style={{ left: pos.x, top: pos.y }}
            onClick={(e) => e.stopPropagation()}
          >
            {items.map((item, i) =>
              item.divider ? (
                <div
                  key={i}
                  className="border-t border-gray-100 dark:border-navy-600 my-1"
                />
              ) : (
                <button
                  key={i}
                  onClick={() => {
                    item.onClick();
                    setVisible(false);
                  }}
                  disabled={item.disabled}
                  className={`w-full text-left px-3 py-2 text-sm flex items-center gap-2.5 transition-colors ${item.disabled
                      ? "text-gray-300 dark:text-gray-600 cursor-not-allowed"
                      : item.danger
                        ? "text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                        : "text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-navy-700"
                    }`}
                >
                  {item.icon && (
                    <span className="w-4 h-4 flex-shrink-0">{item.icon}</span>
                  )}
                  <span className="font-medium">{item.label}</span>
                </button>
              ),
            )}
          </div>,
          document.body,
        )}
    </>
  );
}
