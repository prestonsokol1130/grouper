type Props = {
  count: number;
  page: number;
  onSelect: (page: number) => void;
  /** Adds ‹ › buttons either side of the dots ("Show page arrows"). */
  arrows?: boolean;
  /** The row's height class: 48px on the Launchpad, 32px in the flyout. */
  height?: "h-12" | "h-8";
  /** A 32px-wide column of dots instead of a row, its arrows above and
   * below (the flyout). */
  vertical?: boolean;
};

/** A ‹ or › button: the previous or next page; dimmed at the ends. */
function Arrow({ label, glyph, onClick, disabled }: ArrowProps) {
  return (
    <button
      type="button"
      className="page-arrow subtle focus-ring flex size-6 items-center justify-center rounded-control"
      aria-label={label}
      tabIndex={-1}
      disabled={disabled}
      onClick={onClick}
    >
      <span aria-hidden="true" className="fluent-icon text-[12px]">
        {glyph}
      </span>
    </button>
  );
}

type ArrowProps = { label: string; glyph: string; onClick: () => void; disabled: boolean };

/** Segoe Fluent Icons ChevronUp and ChevronDown. */
const CHEVRON_UP = "\uE70E";
const CHEVRON_DOWN = "\uE70D";

/** Segoe Fluent Icons ChevronLeft and ChevronRight. */
const CHEVRON_LEFT = "";
const CHEVRON_RIGHT = "";

/**
 * One dot per page, centred at the bottom (DESIGN.md section 8), or in a
 * column beside the grid when `vertical`; the current page's dot uses the
 * accent. Clicking a dot jumps to its page; the optional arrows step one
 * page. Nothing shows for a single page. Fills the row or column it sits
 * in.
 */
export function PageDots(props: Props) {
  const { count, page, onSelect, arrows = false, height = "h-12", vertical = false } = props;
  const many = count > 1;
  const layout = vertical ? "w-8 flex-col" : height;
  return (
    <nav
      aria-label="Pages"
      aria-orientation={vertical ? "vertical" : "horizontal"}
      className={`flex ${layout} shrink-0 items-center justify-center gap-1`}
    >
      {many && arrows && (
        <Arrow
          label="Previous page"
          glyph={vertical ? CHEVRON_UP : CHEVRON_LEFT}
          disabled={page <= 0}
          onClick={() => onSelect(page - 1)}
        />
      )}
      {many &&
        Array.from({ length: count }, (_, index) => (
          <button
            key={index}
            type="button"
            className="page-dot focus-ring flex size-4 items-center justify-center rounded-full"
            aria-label={`Page ${index + 1} of ${count}`}
            aria-current={index === page ? "page" : undefined}
            tabIndex={-1}
            onClick={() => onSelect(index)}
          >
            <span aria-hidden="true" className="page-dot-mark block size-2 rounded-full" />
          </button>
        ))}
      {many && arrows && (
        <Arrow
          label="Next page"
          glyph={vertical ? CHEVRON_DOWN : CHEVRON_RIGHT}
          disabled={page >= count - 1}
          onClick={() => onSelect(page + 1)}
        />
      )}
    </nav>
  );
}
