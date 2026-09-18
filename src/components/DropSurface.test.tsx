/**
 * Dragging a shortlist onto the window.
 *
 * These drive the component with the same events a web view sends, because the
 * alternative — checking the helpers and trusting the wiring — is exactly how
 * this broke before: the handlers were registered against the window layer's
 * drag events, which stopped arriving, and nothing failed except the feature.
 */

import { render, screen, act } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DropSurface, carriesFiles, chooseFile, isSpreadsheet } from "./DropSurface";

/** What a web view puts on a file drag; jsdom has no DataTransfer of its own. */
function transfer(files: File[]): DataTransfer {
  return {
    types: ["Files"],
    files: files as unknown as FileList,
    items: files.map((f) => ({ kind: "file", type: f.type })),
    dropEffect: "none",
  } as unknown as DataTransfer;
}

function drag(type: string, dataTransfer: DataTransfer | null): DragEvent {
  const event = new Event(type, { bubbles: true, cancelable: true }) as DragEvent;
  Object.defineProperty(event, "dataTransfer", { value: dataTransfer });
  return event;
}

function sheet(name: string): File {
  return new File(["neo id\nV9H0G6C4\n"], name, { type: "" });
}

describe("dragging a file over the window", () => {
  it("invites the drop, and imports the file", () => {
    const onFile = vi.fn();
    render(<DropSurface onFile={onFile} />);

    act(() => {
      window.dispatchEvent(drag("dragenter", transfer([sheet("Siemens.xlsx")])));
    });
    expect(screen.getByText("Drop to analyse")).toBeInTheDocument();

    const file = sheet("Siemens.xlsx");
    act(() => {
      window.dispatchEvent(drag("drop", transfer([file])));
    });
    expect(onFile).toHaveBeenCalledWith(file);
    expect(screen.queryByText("Drop to analyse")).not.toBeInTheDocument();
  });

  it("claims the drag, so the web view does not open the file itself", () => {
    render(<DropSurface onFile={vi.fn()} />);
    const over = drag("dragover", transfer([sheet("Siemens.xlsx")]));
    act(() => {
      window.dispatchEvent(over);
    });
    expect(over.defaultPrevented).toBe(true);
  });

  it("refuses an unsupported file during the drag, not after it", () => {
    const onFile = vi.fn();
    render(<DropSurface onFile={onFile} />);

    act(() => {
      window.dispatchEvent(drag("drop", transfer([new File([""], "offer.pdf")])));
    });
    expect(screen.getByText("That file won't work")).toBeInTheDocument();
    expect(screen.getByText("offer.pdf")).toBeInTheDocument();
    expect(onFile).not.toHaveBeenCalled();
  });

  it("takes the spreadsheet out of a mixed drag", () => {
    const onFile = vi.fn();
    render(<DropSurface onFile={onFile} />);
    const wanted = sheet("Elgi.xlsx");
    act(() => {
      window.dispatchEvent(drag("drop", transfer([new File([""], "notes.pdf"), wanted])));
    });
    expect(onFile).toHaveBeenCalledWith(wanted);
  });

  it("ignores dragged text and links", () => {
    const onFile = vi.fn();
    render(<DropSurface onFile={onFile} />);
    const text = { types: ["text/plain"], files: [], items: [] } as unknown as DataTransfer;
    const over = drag("dragover", text);
    act(() => {
      window.dispatchEvent(drag("dragenter", text));
      window.dispatchEvent(over);
      window.dispatchEvent(drag("drop", text));
    });
    expect(screen.queryByText("Drop to analyse")).not.toBeInTheDocument();
    expect(over.defaultPrevented).toBe(false);
    expect(onFile).not.toHaveBeenCalled();
  });

  it("does not accept a drop while an import is already running", () => {
    const onFile = vi.fn();
    render(<DropSurface onFile={onFile} disabled />);
    act(() => {
      window.dispatchEvent(drag("drop", transfer([sheet("Siemens.xlsx")])));
    });
    expect(onFile).not.toHaveBeenCalled();
  });

  it("keeps the invitation up while the pointer crosses the interface", () => {
    render(<DropSurface onFile={vi.fn()} />);
    const files = transfer([sheet("Siemens.xlsx")]);
    act(() => {
      window.dispatchEvent(drag("dragenter", files)); // the window
      window.dispatchEvent(drag("dragenter", files)); // a card inside it
      window.dispatchEvent(drag("dragleave", files)); // leaving the card
    });
    expect(screen.getByText("Drop to analyse")).toBeInTheDocument();

    act(() => {
      window.dispatchEvent(drag("dragleave", files)); // and the window
    });
    expect(screen.queryByText("Drop to analyse")).not.toBeInTheDocument();
  });
});

describe("what counts as droppable", () => {
  it("knows the formats nankiv reads", () => {
    for (const name of ["a.xlsx", "A.XLS", "list.csv", "sheet.ods", "macro.xlsm"]) {
      expect(isSpreadsheet(name), name).toBe(true);
    }
    for (const name of ["offer.pdf", "notes.txt", "shortlist", "a.xlsx.zip"]) {
      expect(isSpreadsheet(name), name).toBe(false);
    }
  });

  it("prefers a spreadsheet, and explains with the first file otherwise", () => {
    const sheetFile = sheet("a.xlsx");
    const pdf = new File([""], "b.pdf");
    expect(chooseFile([pdf, sheetFile])).toEqual({ file: sheetFile, ok: true });
    expect(chooseFile([pdf])).toEqual({ file: pdf, ok: false });
    expect(chooseFile([])).toBeNull();
  });

  it("only reacts to drags that carry files", () => {
    expect(carriesFiles(transfer([sheet("a.xlsx")]))).toBe(true);
    expect(carriesFiles({ types: ["text/uri-list"] } as unknown as DataTransfer)).toBe(false);
    expect(carriesFiles(null)).toBe(false);
  });
});
