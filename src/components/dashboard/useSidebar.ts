import { type FocusEvent, type RefObject, useCallback, useEffect, useRef, useState } from "react";

import type { AppSettings } from "../../store/settingsStore";
import { type SidebarLayout, sidebarLayout } from "./sidebarLayout";

/** Hover opens after this, so a pointer passing over the rail doesn't flash the sidebar open. */
const OPEN_DELAY_MS = 150;
/** And closes this long after the pointer leaves, so a slip off the edge doesn't snap it shut. */
const CLOSE_DELAY_MS = 250;

interface SidebarSettings {
  width: AppSettings["sidebar_width"];
  behaviour: AppSettings["sidebar_behaviour"];
  collapsed: boolean;
}

interface Frame {
  windowPx: number;
  rootPx: number;
}

function readFrame(): Frame {
  const rootPx = Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
  return { windowPx: window.innerWidth, rootPx };
}

/**
 * The window's width and the root font size, kept current. A media query can't stand in: it
 * measures rem against the browser default, not the root size the UI scale sets, so this listens
 * for resizes and for the UI scale rewriting the root's inline style.
 */
function useFrame(): Frame {
  const [frame, setFrame] = useState(readFrame);
  useEffect(() => {
    const update = () => setFrame(readFrame());
    window.addEventListener("resize", update);
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["style"] });
    return () => {
      window.removeEventListener("resize", update);
      observer.disconnect();
    };
  }, []);
  return frame;
}

export interface SidebarControls {
  layout: SidebarLayout;
  /** On the sidebar: a pointer-down outside it closes a drawer. */
  asideRef: RefObject<HTMLElement | null>;
  /** Hover and keyboard focus (Expand on hover), and a drawer's focus leaving it; spread on the sidebar. */
  asideHandlers: {
    onPointerEnter: () => void;
    onPointerLeave: () => void;
    onFocus: () => void;
    onBlur: (event: FocusEvent<HTMLElement>) => void;
  };
  /** Open or close the drawer (the sidebar's button when `layout.opensDrawer`). */
  toggle: () => void;
  close: () => void;
}

/**
 * The sidebar's behaviour: where it sits (sidebarLayout) and what opens it over the page. Expand on
 * hover opens after a short delay or on keyboard focus, and closes when the pointer or the focus
 * leaves; a drawer closes when the focus moves on to the page; Esc (outside a dialog) closes any
 * overlay, and a hover closed with Esc stays closed until the pointer leaves. `onEscape` lets the
 * sidebar return the focus to its button.
 */
export function useSidebar(settings: SidebarSettings, onEscape?: () => void): SidebarControls {
  const frame = useFrame();
  const [overlayOpen, setOverlayOpen] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const escaped = useRef(false);
  const asideRef = useRef<HTMLElement | null>(null);
  const hover = settings.behaviour === "hover";
  const layout = sidebarLayout({ ...settings, overlayOpen }, frame);

  // A change of behaviour starts closed: an overlay opened under one behaviour isn't the next's.
  useEffect(() => {
    clearTimeout(timer.current);
    setOverlayOpen(false);
  }, [settings.behaviour]);
  useEffect(() => () => clearTimeout(timer.current), []);

  const schedule = useCallback((open: boolean, delay: number) => {
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setOverlayOpen(open), delay);
  }, []);

  // An overlay the layout no longer draws (the window widened until the sidebar fits in the page)
  // is forgotten, so it doesn't come back open when the window narrows again.
  useEffect(() => {
    if (overlayOpen && !layout.overlay) setOverlayOpen(false);
  }, [overlayOpen, layout.overlay]);

  // The listeners follow the overlay as drawn, not the request: a request the layout ignores must
  // not take Esc from the page (a dialog's, say).
  useEffect(() => {
    if (!layout.overlay) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      // An Esc pressed in a dialog is the dialog's: closing the drawer too would move the focus
      // to the sidebar's button from under it.
      if (event.target instanceof Element && event.target.closest('[role="dialog"]')) return;
      clearTimeout(timer.current);
      escaped.current = hover;
      setOverlayOpen(false);
      onEscape?.();
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [layout.overlay, hover, onEscape]);

  // A drawer closes on a click outside it. No full-screen backdrop catches that click: the only
  // full-screen overlay is ui/Dialog's (design rule 4), and the page should stay usable around it.
  useEffect(() => {
    if (!layout.overlay || !layout.closesOnOutsideClick) return;
    function handlePointerDown(event: PointerEvent) {
      if (asideRef.current?.contains(event.target as Node)) return;
      setOverlayOpen(false);
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [layout.overlay, layout.closesOnOutsideClick]);

  return {
    layout,
    asideRef,
    asideHandlers: {
      onPointerEnter: () => {
        if (hover && !escaped.current) schedule(true, OPEN_DELAY_MS);
      },
      onPointerLeave: () => {
        escaped.current = false;
        if (hover) schedule(false, CLOSE_DELAY_MS);
      },
      onFocus: () => {
        if (!hover || escaped.current) return;
        clearTimeout(timer.current);
        setOverlayOpen(true);
      },
      onBlur: (event) => {
        const next = event.relatedTarget as Node | null;
        if (event.currentTarget.contains(next)) return;
        if (hover) {
          escaped.current = false;
          clearTimeout(timer.current);
          setOverlayOpen(false);
          return;
        }
        // A drawer closes once the focus moves on to the page, so it never sits over the focused
        // element (WCAG 2.4.11). Focus lost to nothing (a click on the drawer's own padding, the
        // window losing focus) leaves it open; a click outside closes it on its own.
        if (layout.overlay && next) setOverlayOpen(false);
      },
    },
    toggle: () => setOverlayOpen((open) => !open),
    close: () => setOverlayOpen(false),
  };
}
