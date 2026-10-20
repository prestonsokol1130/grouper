type Props = {
  count: number;
  page: number;
  onSelect: (page: number) => void;
};

/**
 * One dot per page, centred at the bottom (DESIGN.md section 8); the
 * current page's dot uses the accent. Clicking a dot jumps to its page.
 * Nothing shows for a single page. Fills the 48px row it sits in.
 */
export function PageDots({ count, page, onSelect }: Props) {
  return (
    <nav aria-label="Pages" className="flex h-12 shrink-0 items-center justify-center gap-1">
      {count > 1 &&
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
    </nav>
  );
}
