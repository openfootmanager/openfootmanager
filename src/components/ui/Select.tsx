import {
  Children,
  isValidElement,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ChangeEvent,
  type FocusEventHandler,
  type KeyboardEvent,
  type ReactElement,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown } from "lucide-react";

interface SelectProps {
  selectSize?: "2xs" | "xs" | "sm" | "md";
  variant?: "default" | "subtle" | "muted" | "highlighted" | "placeholder" | "ghost";
  icon?: ReactNode;
  fullWidth?: boolean;
  wrapperClassName?: string;
  className?: string;
  children: ReactNode;
  style?: CSSProperties;
  value?: string | number | readonly string[];
  defaultValue?: string | number | readonly string[];
  onChange?: (event: ChangeEvent<HTMLSelectElement>) => void;
  disabled?: boolean;
  name?: string;
  id?: string;
  required?: boolean;
  title?: string;
  tabIndex?: number;
  autoFocus?: boolean;
  onBlur?: FocusEventHandler<HTMLButtonElement>;
  onFocus?: FocusEventHandler<HTMLButtonElement>;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  "aria-describedby"?: string;
}

interface SelectOption {
  value: string;
  label: ReactNode;
  disabled?: boolean;
}

interface NativeOptionProps {
  value?: string | number | readonly string[];
  disabled?: boolean;
  children?: ReactNode;
}

export function Select({
  selectSize = "md",
  variant = "default",
  icon,
  fullWidth = false,
  wrapperClassName = "",
  className = "",
  children,
  style,
  value,
  defaultValue,
  onChange,
  disabled,
  name,
  id,
  required,
  title,
  tabIndex,
  autoFocus,
  onBlur,
  onFocus,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledBy,
  "aria-describedby": ariaDescribedBy,
}: SelectProps) {
  const listboxId = useId();
  const wrapperRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const controlledValue = value !== undefined ? String(value) : undefined;

  const options = useMemo<SelectOption[]>(() => {
    return Children.toArray(children).flatMap((child) => {
      if (!isValidElement(child) || child.type !== "option") {
        return [];
      }

      const option = child as ReactElement<NativeOptionProps>;

      return [
        {
          value: String(option.props.value ?? ""),
          label: option.props.children,
          disabled: option.props.disabled,
        },
      ];
    });
  }, [children]);

  const [uncontrolledValue, setUncontrolledValue] = useState(() => {
    if (controlledValue !== undefined) {
      return controlledValue;
    }

    if (defaultValue !== undefined) {
      return String(defaultValue);
    }

    return options[0]?.value ?? "";
  });
  const [isOpen, setIsOpen] = useState(false);

  const currentValue = controlledValue ?? uncontrolledValue;
  const selectedOption =
    options.find((option) => option.value === currentValue) ??
    options[0] ??
    null;
  const selectedValue = selectedOption?.value ?? "";
  const enabledOptions = options.filter((option) => !option.disabled);

  useEffect(() => {
    if (controlledValue !== undefined || options.length === 0) {
      return;
    }

    if (!options.some((option) => option.value === uncontrolledValue)) {
      setUncontrolledValue(options[0].value);
    }
  }, [controlledValue, options, uncontrolledValue]);

  useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      // The menu is portaled to <body>, so a press inside it is "outside" the
      // wrapper — it must not dismiss the menu before the option click lands.
      if (
        !wrapperRef.current?.contains(target) &&
        !menuRef.current?.contains(target)
      ) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);

    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, []);

  // Place the portaled menu against the trigger: below by default, flipped
  // above when the viewport bottom would clip it (e.g. the GK slot at the foot
  // of the tactics pitch, issue #282). Re-derived on scroll/resize so the menu
  // tracks a trigger inside scrollable panels.
  useLayoutEffect(() => {
    if (!isOpen) {
      return;
    }

    const place = () => {
      const trigger = wrapperRef.current;
      const menu = menuRef.current;
      if (!trigger || !menu) {
        return;
      }

      const rect = trigger.getBoundingClientRect();
      const margin = 8;
      const gap = 4;
      // The menu may grow past the trigger (long option labels) but never
      // narrower than it, so short-option dropdowns keep their current look.
      menu.style.minWidth = `${rect.width}px`;
      menu.style.maxWidth = `min(18rem, calc(100vw - ${margin * 2}px))`;

      // Measure at the list's default cap, then take the roomier side when
      // neither fully fits, shrinking the scrollable list so the chosen side
      // never clips.
      const list = menu.querySelector<HTMLElement>('[role="listbox"]');
      if (list) {
        list.style.maxHeight = "";
      }
      const availableBelow = window.innerHeight - margin - rect.bottom - gap;
      const availableAbove = rect.top - gap - margin;
      const naturalHeight = menu.offsetHeight;
      const openUp =
        naturalHeight > availableBelow && availableAbove > availableBelow;
      const available = openUp ? availableAbove : availableBelow;
      if (list && naturalHeight > available) {
        const chrome = naturalHeight - list.offsetHeight;
        list.style.maxHeight = `${Math.max(80, available - chrome)}px`;
      }

      const menuHeight = menu.offsetHeight;
      const menuWidth = menu.offsetWidth;
      menu.style.top = openUp
        ? `${rect.top - gap - menuHeight}px`
        : `${rect.bottom + gap}px`;
      menu.style.left = `${Math.max(
        margin,
        Math.min(rect.left, window.innerWidth - menuWidth - margin),
      )}px`;
    };

    let frame = 0;
    const schedulePlace = () => {
      if (frame) {
        return;
      }
      frame = requestAnimationFrame(() => {
        frame = 0;
        place();
      });
    };
    const handleScroll = (event: Event) => {
      // Scrolling the options list itself doesn't move the trigger.
      if (menuRef.current?.contains(event.target as Node)) {
        return;
      }
      schedulePlace();
    };

    place();
    window.addEventListener("resize", schedulePlace);
    window.addEventListener("scroll", handleScroll, true);

    return () => {
      if (frame) {
        cancelAnimationFrame(frame);
      }
      window.removeEventListener("resize", schedulePlace);
      window.removeEventListener("scroll", handleScroll, true);
    };
  }, [isOpen, options]);

  const handleSelect = (nextValue: string) => {
    if (controlledValue === undefined) {
      setUncontrolledValue(nextValue);
    }

    onChange?.({
      target: { value: nextValue },
      currentTarget: { value: nextValue },
    } as ChangeEvent<HTMLSelectElement>);

    setIsOpen(false);
  };

  const toggleOpen = () => {
    if (disabled || options.length === 0) {
      return;
    }

    setIsOpen((open) => !open);
  };

  const moveSelection = (direction: 1 | -1) => {
    if (enabledOptions.length === 0) {
      return;
    }

    const currentIndex = enabledOptions.findIndex(
      (option) => option.value === selectedValue,
    );
    const baseIndex = currentIndex >= 0 ? currentIndex : 0;
    const nextIndex =
      (baseIndex + direction + enabledOptions.length) % enabledOptions.length;
    handleSelect(enabledOptions[nextIndex].value);
  };

  const handleTriggerKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveSelection(1);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveSelection(-1);
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      setIsOpen(true);
      return;
    }

    if (event.key === "Escape") {
      setIsOpen(false);
    }
  };

  const base =
    "rounded-lg border transition-all focus:outline-none focus:ring-2 focus:ring-primary-500/30 disabled:opacity-50 disabled:cursor-not-allowed";

  const variants = {
    default:
      "bg-white dark:bg-navy-800 border-gray-200 dark:border-navy-600 text-gray-700 dark:text-gray-200",
    subtle:
      "bg-gray-100 dark:bg-navy-700 border-gray-200 dark:border-navy-600 text-gray-600 dark:text-gray-300",
    muted:
      "bg-gray-50 dark:bg-navy-700 border-gray-200 dark:border-navy-600 text-gray-700 dark:text-gray-300",
    highlighted:
      "bg-primary-50 dark:bg-primary-500/10 border-primary-300 dark:border-primary-500/40 text-primary-700 dark:text-primary-300 font-bold",
    placeholder:
      "bg-gray-50 dark:bg-navy-700 border-gray-200 dark:border-navy-600 text-gray-400 dark:text-gray-500",
    ghost:
      "bg-white/10 border-white/10 text-white hover:bg-white/20",
  };

  const sizes = {
    "2xs": "py-0.5 text-[9px]",
    xs: "py-0.5 text-[10px]",
    sm: "py-1.5 text-xs",
    md: "py-2 text-sm",
  };

  const leftPadding = icon
    ? { "2xs": "pl-6", xs: "pl-7", sm: "pl-8", md: "pl-9" }[selectSize]
    : { "2xs": "pl-1.5", xs: "pl-3", sm: "pl-3", md: "pl-3" }[selectSize];

  const rightPadding = { "2xs": "pr-4", xs: "pr-6", sm: "pr-8", md: "pr-9" }[selectSize];
  const iconInset = { "2xs": "left-1.5", xs: "left-2", sm: "left-2.5", md: "left-3" }[selectSize];
  const chevronInset = { "2xs": "right-1", xs: "right-2", sm: "right-2.5", md: "right-3" }[
    selectSize
  ];
  const chevronSize = { "2xs": "w-2.5 h-2.5", xs: "w-3 h-3", sm: "w-4 h-4", md: "w-4 h-4" }[
    selectSize
  ];
  const optionTextSize = { "2xs": "text-[9px]", xs: "text-[10px]", sm: "text-xs", md: "text-sm" }[
    selectSize
  ];

  return (
    <div
      ref={wrapperRef}
      className={`relative ${fullWidth ? "w-full" : ""} ${wrapperClassName}`}
    >
      {name ? (
        <input
          type="hidden"
          name={name}
          value={selectedValue}
          disabled={disabled}
        />
      ) : null}
      {icon ? (
        <span
          className={`pointer-events-none absolute inset-y-0 ${iconInset} flex items-center text-gray-400 dark:text-gray-500`}
          aria-hidden="true"
        >
          <span className="[&>svg]:w-4 [&>svg]:h-4">{icon}</span>
        </span>
      ) : null}
      <button
        type="button"
        id={id}
        title={title}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        aria-describedby={ariaDescribedBy}
        role="combobox"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={listboxId}
        tabIndex={tabIndex}
        autoFocus={autoFocus}
        className={`${base} ${variants[variant]} ${sizes[selectSize]} ${leftPadding} ${rightPadding} ${fullWidth ? "w-full" : ""} ${className} flex items-center justify-between text-left`}
        style={style}
        onClick={(event) => {
          event.stopPropagation();
          toggleOpen();
        }}
        onKeyDown={handleTriggerKeyDown}
        onBlur={onBlur}
        onFocus={onFocus}
      >
        <span className="truncate">{selectedOption?.label ?? ""}</span>
      </button>
      <span
        className={`pointer-events-none absolute inset-y-0 ${chevronInset} flex items-center text-gray-400 dark:text-gray-500 transition-transform ${isOpen ? "rotate-180" : ""}`}
        aria-hidden="true"
      >
        <ChevronDown className={chevronSize} />
      </span>

      {isOpen ? createPortal(
        <div
          ref={menuRef}
          className="fixed z-50 w-max overflow-hidden rounded-xl border border-gray-200 bg-white shadow-xl dark:border-navy-600 dark:bg-navy-800"
        >
          <div
            id={listboxId}
            role="listbox"
            aria-required={required}
            className="max-h-60 overflow-y-auto p-1"
          >
            {options.map((option) => {
              const isSelected = option.value === currentValue;

              return (
                <button
                  key={option.value}
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  disabled={option.disabled}
                  className={`${optionTextSize} flex w-full items-center justify-between rounded-lg px-3 py-2 text-left transition-colors ${isSelected ? "bg-primary-50 text-primary-600 dark:bg-primary-500/10 dark:text-primary-400" : "text-gray-700 hover:bg-gray-50 dark:text-gray-200 dark:hover:bg-navy-700"} ${option.disabled ? "cursor-not-allowed opacity-50" : ""}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    if (!option.disabled) {
                      handleSelect(option.value);
                    }
                  }}
                >
                  <span className="truncate">{option.label}</span>
                  {isSelected ? (
                    <Check className="ml-2 h-4 w-4 shrink-0" />
                  ) : null}
                </button>
              );
            })}
          </div>
        </div>,
        document.body,
      ) : null}
    </div>
  );
}
