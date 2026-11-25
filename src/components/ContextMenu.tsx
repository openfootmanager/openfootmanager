import {
  cloneElement,
  forwardRef,
  isValidElement,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  useCallback,
} from "react";
import { createPortal } from "react-dom";

import { contextMenuPoint } from "../lib/contextMenuPoint";

export interface ContextMenuItem {
  label: string;
  icon?: React.ReactNode;
  onClick?: () => void;
  danger?: boolean;
  urgent?: boolean;
  disabled?: boolean;
  divider?: boolean;
  type?: "label";
}

export interface ContextMenuHandle {
  open: (x: number, y: number) => void;
}

interface ContextMenuProps {
  items: ContextMenuItem[];
  children: React.ReactNode;
  onOpenChange?: (open: boolean) => void;
}

const ContextMenu = forwardRef<ContextMenuHandle, ContextMenuProps>(function ContextMenu(
  { items, children, onOpenChange },
  ref,
) {
  const [visible, setVisible] = useState(false);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const menuRef = useRef<HTMLDivElement>(null);
  const instanceId = useRef(Math.random().toString(36));
  const triggerRef = useRef<HTMLElement | null>(null);
  const onOpenChangeRef = useRef(onOpenChange);

  useEffect(() => {
    onOpenChangeRef.current = onOpenChange;
  }, [onOpenChange]);

  // Whether the menu is open, as last set: read and written outside a state updater, because an
  // updater runs while this component renders (and twice under StrictMode), and the owner's
  // `onOpenChange` sets the owner's state.
  const openRef = useRef(false);

  const setOpen = useCallback((next: boolean) => {
    // Notify only on real transitions so consumers wiring aria-expanded on
    // their trigger don't get spurious "closed" pings when closeFromOther
    // fires while we're already closed.
    if (openRef.current === next) return;
    openRef.current = next;
    setVisible(next);
    onOpenChangeRef.current?.(next);
  }, []);

  const closeAndRestoreFocus = useCallback(() => {
    setOpen(false);
    triggerRef.current?.focus();
  }, [setOpen]);

  const openAt = useCallback(
    (x: number, y: number) => {
      window.dispatchEvent(new CustomEvent("close-context-menus", { detail: instanceId.current }));
      triggerRef.current = document.activeElement as HTMLElement | null;
      const clampedX = Math.min(x, window.innerWidth - 200);
      const clampedY = Math.min(y, window.innerHeight - 300);
      setPos({ x: clampedX, y: clampedY });
      setOpen(true);
    },
    [setOpen],
  );

  useImperativeHandle(ref, () => ({ open: openAt }), [openAt]);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      // From the keyboard, under the focused control: the trigger may be a box-less wrapper.
      const point = contextMenuPoint(e, e.target instanceof Element ? e.target : e.currentTarget);
      openAt(point.x, point.y);
    },
    [openAt],
  );

  useEffect(() => {
    const closeFromOther = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail !== instanceId.current) setOpen(false);
    };
    window.addEventListener("close-context-menus", closeFromOther);
    return () => window.removeEventListener("close-context-menus", closeFromOther);
  }, [setOpen]);

  useEffect(() => {
    if (!visible) return;
    const close = () => setOpen(false);
    const closeOnEscape = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // Claimed in the capture phase so a dialog around the menu sees it as handled and stays open.
      e.preventDefault();
      closeAndRestoreFocus();
    };
    window.addEventListener("click", close);
    window.addEventListener("scroll", close, true);
    window.addEventListener("keydown", closeOnEscape, true);
    // Move keyboard focus into the menu so arrow-key navigation / Enter
    // selection is available without tabbing through the rest of the page.
    const firstItem = menuRef.current?.querySelector<HTMLButtonElement>(
      'button[role="menuitem"]:not([disabled])',
    );
    firstItem?.focus();
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("keydown", closeOnEscape, true);
    };
  }, [visible, setOpen, closeAndRestoreFocus]);

  /**
   * ArrowDown/ArrowUp move focus through the enabled items (wrapping), Home and End jump to the ends;
   * Enter and Space are the focused button's own click. A disabled item is not focusable, so the
   * enabled ones are the whole list.
   */
  const moveFocus = useCallback((e: React.KeyboardEvent) => {
    const keys = ["ArrowDown", "ArrowUp", "Home", "End"];
    if (!keys.includes(e.key)) return;
    const items = [
      ...(menuRef.current?.querySelectorAll<HTMLButtonElement>(
        'button[role="menuitem"]:not([disabled])',
      ) ?? []),
    ];
    if (items.length === 0) return;
    e.preventDefault();
    const current = items.findIndex((item) => item === document.activeElement);
    const last = items.length - 1;
    let next: number;
    if (e.key === "Home") next = 0;
    else if (e.key === "End") next = last;
    else if (e.key === "ArrowDown") next = current >= last ? 0 : current + 1;
    else next = current <= 0 ? last : current - 1;
    items[next]?.focus();
  }, []);

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
            role="menu"
            className="fixed z-50 min-w-[180px] rounded-lg border border-line bg-panel py-1 shadow-card animate-in fade-in duration-100"
            style={{ left: pos.x, top: pos.y }}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={moveFocus}
          >
            {items.map((item, i) =>
              item.divider ? (
                <div key={i} className="my-1 border-t border-line" />
              ) : item.type === "label" ? (
                <div
                  key={i}
                  className="flex items-center gap-2.5 px-3 py-2 text-xs font-medium text-fg-muted"
                >
                  {item.icon && (
                    <span className="h-4 w-4 flex-shrink-0 text-warning">{item.icon}</span>
                  )}
                  <span>{item.label}</span>
                </div>
              ) : (
                <button
                  type="button"
                  key={i}
                  role="menuitem"
                  onClick={() => {
                    item.onClick?.();
                    closeAndRestoreFocus();
                  }}
                  disabled={item.disabled}
                  className={`flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-focus ${
                    item.disabled
                      ? "cursor-not-allowed text-fg-subtle"
                      : item.danger
                        ? "text-danger hover:bg-danger-tint"
                        : item.urgent
                          ? "text-warning hover:bg-warning-tint"
                          : "text-fg hover:bg-hover"
                  }`}
                >
                  {item.icon && <span className="w-4 h-4 flex-shrink-0">{item.icon}</span>}
                  <span className="font-medium">{item.label}</span>
                </button>
              ),
            )}
          </div>,
          document.body,
        )}
    </>
  );
});

export default ContextMenu;
