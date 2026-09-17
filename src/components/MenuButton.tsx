/**
 * A button that opens a short list of choices.
 *
 * Used where one action has a couple of variants — "download names" as Excel or
 * as CSV — so the variants live behind the action instead of doubling the
 * number of buttons on screen. Behaves like a native pop-up menu: arrow keys
 * move, Enter chooses, Escape and clicking elsewhere dismiss, and focus returns
 * to the button afterwards.
 */

import { useEffect, useId, useRef, useState } from "react";
import { Icon } from "./Icon";

export interface MenuChoice {
  id: string;
  label: string;
  hint?: string;
}

export function MenuButton({
  label,
  choices,
  onChoose,
}: {
  label: string;
  choices: MenuChoice[];
  onChoose: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const wrap = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const items = useRef<(HTMLButtonElement | null)[]>([]);
  const menuId = useId();

  useEffect(() => {
    if (!open) return;
    items.current[active]?.focus();
  }, [open, active]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!wrap.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [open]);

  function close() {
    setOpen(false);
    button.current?.focus();
  }

  function choose(id: string) {
    close();
    onChoose(id);
  }

  function onMenuKey(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => (i + 1) % choices.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => (i - 1 + choices.length) % choices.length);
    } else if (e.key === "Tab") {
      setOpen(false);
    }
  }

  return (
    <div className="menu-wrap" ref={wrap}>
      <button
        ref={button}
        className="btn"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => {
          setActive(0);
          setOpen((v) => !v);
        }}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown" && !open) {
            e.preventDefault();
            setActive(0);
            setOpen(true);
          }
        }}
      >
        {label}
        <Icon name="chevronRight" size={12} className="menu-chevron" />
      </button>

      {open && (
        <div
          className="menu-pop"
          role="menu"
          id={menuId}
          aria-label={label}
          onKeyDown={onMenuKey}
        >
          {choices.map((c, i) => (
            <button
              key={c.id}
              ref={(el) => {
                items.current[i] = el;
              }}
              role="menuitem"
              className="menu-item"
              tabIndex={i === active ? 0 : -1}
              onMouseEnter={() => setActive(i)}
              onClick={() => choose(c.id)}
            >
              <span>{c.label}</span>
              {c.hint && <span className="menu-hint">{c.hint}</span>}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
