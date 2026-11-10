import type { SVGProps } from "react";

/**
 * The few icons the chrome needs.
 *
 * They are inline SVG with `stroke="currentColor"` rather than emoji. Emoji
 * render in a fixed colour from whichever font the OS picks, so they keep their
 * own palette through a theme switch: a greyed-out disabled row still shows a
 * bright coloured glyph, and a muted label sits beside a saturated picture. An
 * icon drawn in `currentColor` takes the colour of the text it belongs to, so it
 * fades and dims with the state it is supposed to describe.
 *
 * Each one is decorative by default — the control it sits in carries the name —
 * so `aria-hidden` is set here instead of at every call site.
 */
function Icon({
  path,
  className = "h-4 w-4",
  ...rest
}: { path: string; className?: string } & SVGProps<SVGSVGElement>) {
  return (
    <svg
      aria-hidden
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      {...rest}
    >
      <path d={path} />
    </svg>
  );
}

export function FolderIcon(props: { className?: string }) {
  return <Icon path="M1.5 4.5A1 1 0 0 1 2.5 3.5h3.6l1.4 1.6h6.5a1 1 0 0 1 1 1v5.9a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z" {...props} />;
}

export function TrashIcon(props: { className?: string }) {
  return (
    <Icon
      path="M2.5 4.5h11M6 4.5V3a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5v1.5M4 4.5l.6 8.2a.8.8 0 0 0 .8.8h5.2a.8.8 0 0 0 .8-.8L12 4.5"
      {...props}
    />
  );
}

export function ScalesIcon(props: { className?: string }) {
  return (
    <Icon
      path="M8 2.5v11M4 13.5h8M8 4.5 4 6.5m4-2 4 2M4 6.5 2 11a2.2 2.2 0 0 0 4 0zm8 0-2 4.5a2.2 2.2 0 0 0 4 0z"
      {...props}
    />
  );
}
