import {
  Children,
  isValidElement,
  useEffect,
  useId,
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
import { Check, ChevronDown } from "lucide-react";

interface SelectProps {
  selectSize?: "xs" | "sm" | "md";
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
      if (!wrapperRef.current?.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);

    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, []);

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
    xs: "py-0.5 text-[10px]",
    sm: "py-1.5 text-xs",
    md: "py-2 text-sm",
  };

  const leftPadding = icon
    ? { xs: "pl-7", sm: "pl-8", md: "pl-9" }[selectSize]
    : "pl-3";

  const rightPadding = { xs: "pr-6", sm: "pr-8", md: "pr-9" }[selectSize];
  const iconInset = { xs: "left-2", sm: "left-2.5", md: "left-3" }[selectSize];
  const chevronInset = { xs: "right-2", sm: "right-2.5", md: "right-3" }[
    selectSize
  ];
  const chevronSize = { xs: "w-3 h-3", sm: "w-4 h-4", md: "w-4 h-4" }[
    selectSize
  ];
  const optionTextSize = { xs: "text-[10px]", sm: "text-xs", md: "text-sm" }[
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

      {isOpen ? (
        <div className="absolute left-0 right-0 top-full z-50 mt-1 overflow-hidden rounded-xl border border-gray-200 bg-white shadow-xl dark:border-navy-600 dark:bg-navy-800">
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
        </div>
      ) : null}
    </div>
  );
}
