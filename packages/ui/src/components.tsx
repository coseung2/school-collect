import {
  forwardRef,
  useId,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
  type KeyboardEvent,
  type LiHTMLAttributes,
  type ReactNode,
  type SVGProps,
} from "react";

export type IconName =
  | "activity"
  | "alert"
  | "briefcase"
  | "chevron-down"
  | "check"
  | "inbox"
  | "lock"
  | "refresh"
  | "settings"
  | "school"
  | "sliders"
  | "wifi-off";

function cx(
  ...values: Array<string | false | null | undefined>
): string {
  return values.filter(Boolean).join(" ");
}

export interface IconProps extends Omit<SVGProps<SVGSVGElement>, "name"> {
  name: IconName;
  size?: number;
  label?: string;
}

export function Icon({
  name,
  size = 18,
  label,
  className,
  ...props
}: IconProps) {
  const common = {
    fill: "none",
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.8,
  };

  return (
    <svg
      aria-hidden={label ? undefined : true}
      aria-label={label}
      className={cx("sc-icon", className)}
      height={size}
      role={label ? "img" : undefined}
      viewBox="0 0 24 24"
      width={size}
      {...props}
    >
      {name === "activity" && (
        <>
          <path {...common} d="M3 12h4l2.2-6 4.1 12 2.2-6H21" />
        </>
      )}
      {name === "alert" && (
        <>
          <path {...common} d="m12 3 9 16H3L12 3Z" />
          <path {...common} d="M12 9v4m0 3h.01" />
        </>
      )}
      {name === "briefcase" && (
        <>
          <rect {...common} height="13" rx="2" width="18" x="3" y="7" />
          <path {...common} d="M9 7V5h6v2m-12 5h18m-9 0v3" />
        </>
      )}
      {name === "chevron-down" && <path {...common} d="m6 9 6 6 6-6" />}
      {name === "check" && <path {...common} d="m5 12 4 4L19 6" />}
      {name === "inbox" && (
        <>
          <path {...common} d="M4 5h16l2 10v3a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-3L4 5Z" />
          <path {...common} d="M2 14h5l2 3h6l2-3h5" />
        </>
      )}
      {name === "lock" && (
        <>
          <rect {...common} height="10" rx="2" width="14" x="5" y="10" />
          <path {...common} d="M8 10V7a4 4 0 0 1 8 0v3m-4 4v2" />
        </>
      )}
      {name === "refresh" && (
        <>
          <path {...common} d="M20 11a8 8 0 0 0-14.7-4L3 10m0 0V5m0 5h5" />
          <path {...common} d="M4 13a8 8 0 0 0 14.7 4L21 14m0 0v5m0-5h-5" />
        </>
      )}
      {name === "school" && (
        <>
          <path {...common} d="m3 10 9-5 9 5-9 5-9-5Z" />
          <path {...common} d="M7 12v5c2.8 2 7.2 2 10 0v-5M21 10v6" />
        </>
      )}
      {name === "settings" && (
        <>
          <path {...common} d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Z" />
          <path
            {...common}
            d="m19.4 15 .1.1a1.8 1.8 0 0 1-2.5 2.5l-.1-.1a1.8 1.8 0 0 0-3.1 1.3v.2a1.8 1.8 0 0 1-3.6 0v-.2a1.8 1.8 0 0 0-3.1-1.3l-.1.1a1.8 1.8 0 1 1-2.5-2.5l.1-.1A1.8 1.8 0 0 0 3.3 12H3.1a1.8 1.8 0 0 1 0-3.6h.2a1.8 1.8 0 0 0 1.3-3.1l-.1-.1A1.8 1.8 0 1 1 7 2.7l.1.1a1.8 1.8 0 0 0 3.1-1.3v-.2a1.8 1.8 0 0 1 3.6 0v.2a1.8 1.8 0 0 0 3.1 1.3l.1-.1a1.8 1.8 0 1 1 2.5 2.5l-.1.1A1.8 1.8 0 0 0 20.7 9h.2a1.8 1.8 0 0 1 0 3.6h-.2a1.8 1.8 0 0 0-1.3 2.4Z"
          />
        </>
      )}
      {name === "sliders" && (
        <>
          <path {...common} d="M4 6h16M4 12h16M4 18h16" />
          <path {...common} d="M8 4v4m8 2v4M10 16v4" />
        </>
      )}
      {name === "wifi-off" && (
        <>
          <path {...common} d="M3 8a15 15 0 0 1 18 0M6 12a10 10 0 0 1 9.4-1.5M9.5 16a5 5 0 0 1 4.8-.1M12 20h.01" />
          <path {...common} d="M3 3 21 21" />
        </>
      )}
    </svg>
  );
}

export type ButtonVariant = "primary" | "secondary" | "quiet" | "danger";
export type ButtonSize = "small" | "medium";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children?: ReactNode;
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  leadingIcon?: IconName;
  trailingIcon?: IconName;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  function Button(
    {
      children,
      className,
      disabled,
      leadingIcon,
      loading = false,
      size = "medium",
      trailingIcon,
      type: buttonType = "button",
      variant = "primary",
      ...props
    },
    ref,
  ) {
    return (
      <button
        aria-busy={loading || undefined}
        className={cx(
          "sc-button",
          `sc-button--${size}`,
          `sc-button--${variant}`,
          className,
        )}
        disabled={disabled || loading}
        ref={ref}
        type={buttonType}
        {...props}
      >
        {loading ? <span aria-hidden="true" className="sc-spinner" /> : null}
        {!loading && leadingIcon ? <Icon name={leadingIcon} size={16} /> : null}
        <span>{children}</span>
        {!loading && trailingIcon ? <Icon name={trailingIcon} size={16} /> : null}
      </button>
    );
  },
);

export type StatusTone = "neutral" | "info" | "success" | "warning" | "danger";

export interface StatusProps extends HTMLAttributes<HTMLSpanElement> {
  children?: ReactNode;
  tone?: StatusTone;
  dot?: boolean;
}

export function Status({
  children,
  className,
  dot = true,
  tone = "neutral",
  ...props
}: StatusProps) {
  return (
    <span
      className={cx("sc-status", `sc-status--${tone}`, className)}
      {...props}
    >
      {dot ? <span aria-hidden="true" className="sc-status__dot" /> : null}
      <span>{children}</span>
    </span>
  );
}

export type CardVariant = "default" | "attention" | "featured";

export interface CardProps extends HTMLAttributes<HTMLElement> {
  children?: ReactNode;
  variant?: CardVariant;
}

export function Card({
  children,
  className,
  variant = "default",
  ...props
}: CardProps) {
  return (
    <section
      className={cx("sc-card", `sc-card--${variant}`, className)}
      {...props}
    >
      {children}
    </section>
  );
}

export interface ListSurfaceProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

export function ListSurface({
  children,
  className,
  ...props
}: ListSurfaceProps) {
  return (
    <div className={cx("sc-list-surface", className)} {...props}>
      <ul className="sc-list-surface__list">{children}</ul>
    </div>
  );
}

export interface ListRowProps
  extends Omit<LiHTMLAttributes<HTMLLIElement>, "title"> {
  title: ReactNode;
  description?: ReactNode;
  leading?: ReactNode;
  meta?: ReactNode;
  status?: ReactNode;
  action?: ReactNode;
  urgent?: boolean;
}

export function ListRow({
  action,
  className,
  description,
  leading,
  meta,
  status,
  title,
  urgent = false,
  ...props
}: ListRowProps) {
  return (
    <li
      className={cx("sc-list-row", urgent && "sc-list-row--urgent", className)}
      {...props}
    >
      {leading ? <div className="sc-list-row__leading">{leading}</div> : null}
      <div className="sc-list-row__body">
        <div className="sc-list-row__heading">
          <p className="sc-list-row__title">{title}</p>
          {status}
        </div>
        {description ? (
          <p className="sc-list-row__description">{description}</p>
        ) : null}
      </div>
      {meta ? <div className="sc-list-row__meta">{meta}</div> : null}
      {action ? <div className="sc-list-row__action">{action}</div> : null}
    </li>
  );
}

export interface TabItem {
  id: string;
  label: string;
  disabled?: boolean;
}

export interface TabsProps {
  items: TabItem[];
  value: string;
  onChange: (id: string) => void;
  label?: string;
}

export function Tabs({
  items,
  label = "탭",
  onChange,
  value,
}: TabsProps) {
  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, id: string) {
    const enabled = items.filter((item) => !item.disabled);
    const index = enabled.findIndex((item) => item.id === id);
    if (index < 0) return;

    let nextIndex: number;
    switch (event.key) {
      case "ArrowRight":
        nextIndex = (index + 1) % enabled.length;
        break;
      case "ArrowLeft":
        nextIndex = (index - 1 + enabled.length) % enabled.length;
        break;
      case "Home":
        nextIndex = 0;
        break;
      case "End":
        nextIndex = enabled.length - 1;
        break;
      default:
        return;
    }

    event.preventDefault();
    const nextId = enabled[nextIndex].id;
    onChange(nextId);
    const buttons = event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>("[role=tab]");
    buttons?.[items.findIndex((item) => item.id === nextId)]?.focus();
  }

  return (
    <div aria-label={label} className="sc-tabs" role="tablist">
      {items.map((item) => (
        <button
          aria-selected={item.id === value}
          className={cx("sc-tab", item.id === value && "sc-tab--active")}
          disabled={item.disabled}
          key={item.id}
          onKeyDown={(event) => handleKeyDown(event, item.id)}
          onClick={() => onChange(item.id)}
          role="tab"
          tabIndex={item.id === value ? 0 : -1}
          type="button"
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}

export interface DataTableColumn<T> {
  id: string;
  header: ReactNode;
  cell: (row: T) => ReactNode;
  align?: "start" | "center" | "end";
}

export interface DataTableProps<T> {
  columns: DataTableColumn<T>[];
  rows: T[];
  rowKey: (row: T, index: number) => string;
  caption?: string;
  emptyLabel?: string;
}

export function DataTable<T>({
  caption,
  columns,
  emptyLabel = "표시할 데이터가 없습니다.",
  rowKey,
  rows,
}: DataTableProps<T>) {
  return (
    <div className="sc-table-wrap">
      <table className="sc-table">
        {caption ? <caption>{caption}</caption> : null}
        <thead>
          <tr>
            {columns.map((column) => (
              <th
                className={`sc-table__cell--${column.align ?? "start"}`}
                key={column.id}
                scope="col"
              >
                {column.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.length > 0 ? (
            rows.map((row, index) => (
              <tr key={rowKey(row, index)}>
                {columns.map((column) => (
                  <td
                    className={`sc-table__cell--${column.align ?? "start"}`}
                    key={column.id}
                  >
                    {column.cell(row)}
                  </td>
                ))}
              </tr>
            ))
          ) : (
            <tr>
              <td className="sc-table__empty" colSpan={columns.length}>
                {emptyLabel}
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

export interface FormFieldProps extends HTMLAttributes<HTMLDivElement> {
  label: string;
  htmlFor?: string;
  hint?: string;
  error?: string;
  required?: boolean;
  children: ReactNode;
}

export function FormField({
  children,
  className,
  error,
  hint,
  htmlFor,
  label,
  required = false,
  ...props
}: FormFieldProps) {
  const hintId = useId();
  const errorId = useId();

  return (
    <div className={cx("sc-form-field", className)} {...props}>
      <label className="sc-form-field__label" htmlFor={htmlFor}>
        {label}
        {required ? <span aria-hidden="true"> *</span> : null}
      </label>
      {children}
      {error ? (
        <p className="sc-form-field__error" id={errorId} role="alert">
          {error}
        </p>
      ) : hint ? (
        <p className="sc-form-field__hint" id={hintId}>
          {hint}
        </p>
      ) : null}
    </div>
  );
}

export interface NavigationItem {
  id: string;
  label: string;
  icon: IconName;
  group?: string;
  disabled?: boolean;
}

export interface SidebarProps {
  brand?: string;
  subtitle?: string;
  items: NavigationItem[];
  activeId: string;
  onSelect: (id: string) => void;
  footer?: ReactNode;
}

export function Sidebar({
  activeId,
  brand = "School Collect",
  footer,
  items,
  onSelect,
  subtitle = "",
}: SidebarProps) {
  return (
    <aside className="sc-sidebar">
      <div className="sc-brand">
        <span aria-hidden="true" className="sc-brand__mark">
          SC
        </span>
        <span className="sc-brand__copy">
          <strong>{brand}</strong>
          <span>{subtitle}</span>
        </span>
      </div>
      <nav aria-label="주 메뉴" className="sc-sidebar__nav">
        {items.map((item, index) => {
          const active = item.id === activeId;

          return (
            <div className="sc-nav-entry" key={item.id}>
              {item.group && (index === 0 || items[index - 1].group !== item.group) ? (
                <span className="sc-nav-group">{item.group}</span>
              ) : null}
              <button
                aria-current={active ? "page" : undefined}
                className={cx("sc-nav-item", active && "sc-nav-item--active")}
                disabled={item.disabled}
                onClick={() => onSelect(item.id)}
                type="button"
              >
                <span>{item.label}</span>
              </button>
            </div>
          );
        })}
      </nav>
      {footer ? <div className="sc-sidebar__footer">{footer}</div> : null}
    </aside>
  );
}

export interface HeaderProps {
  eyebrow?: string;
  title: string;
  description?: string;
  status?: ReactNode;
  actions?: ReactNode;
}

export function Header({
  actions,
  description,
  eyebrow,
  status,
  title,
}: HeaderProps) {
  return (
    <header className="sc-header">
      <div className="sc-header__copy">
        {eyebrow ? <p className="sc-eyebrow">{eyebrow}</p> : null}
        <div className="sc-header__title-row">
          <h1>{title}</h1>
          {status}
        </div>
        {description ? <p className="sc-header__description">{description}</p> : null}
      </div>
      {actions ? <div className="sc-header__actions">{actions}</div> : null}
    </header>
  );
}

export interface AppShellProps extends HeaderProps {
  navigation: NavigationItem[];
  activeNavigationId: string;
  onNavigationChange: (id: string) => void;
  sidebarFooter?: ReactNode;
  banner?: ReactNode;
  children: ReactNode;
}

export function AppShell({
  activeNavigationId,
  banner,
  children,
  navigation,
  onNavigationChange,
  sidebarFooter,
  ...headerProps
}: AppShellProps) {
  return (
    <div className="sc-shell">
      <Sidebar
        activeId={activeNavigationId}
        footer={sidebarFooter}
        items={navigation}
        onSelect={onNavigationChange}
      />
      <div className="sc-shell__main">
        <Header {...headerProps} />
        {banner ? <div className="sc-shell__banner">{banner}</div> : null}
        <main className="sc-content">{children}</main>
      </div>
    </div>
  );
}

export interface StatePanelProps extends HTMLAttributes<HTMLDivElement> {
  title: string;
  description: string;
  action?: ReactNode;
  icon: IconName;
}

function StatePanel({
  action,
  className,
  description,
  icon,
  title,
  ...props
}: StatePanelProps) {
  return (
    <div className={cx("sc-state-panel", className)} {...props}>
      <span aria-hidden="true" className="sc-state-panel__icon">
        <Icon name={icon} size={20} />
      </span>
      <div>
        <h2>{title}</h2>
        <p>{description}</p>
        {action ? <div className="sc-state-panel__action">{action}</div> : null}
      </div>
    </div>
  );
}

export function EmptyState(props: Omit<StatePanelProps, "icon">) {
  return <StatePanel icon="inbox" {...props} />;
}

export function LoadingState(props: Omit<StatePanelProps, "icon">) {
  return <StatePanel aria-live="polite" icon="activity" {...props} />;
}

export function ErrorState(props: Omit<StatePanelProps, "icon">) {
  return <StatePanel icon="alert" {...props} />;
}

export function PermissionState(props: Omit<StatePanelProps, "icon">) {
  return <StatePanel icon="lock" {...props} />;
}

export function OfflineState(props: Omit<StatePanelProps, "icon">) {
  return <StatePanel icon="wifi-off" {...props} />;
}
