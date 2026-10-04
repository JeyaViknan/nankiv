import { describe, expect, it } from "vitest";
import { isTheName, lateNightLine, wrongFileReply } from "./eggs";

describe("the name", () => {
  it("answers to viknan, however it's typed", () => {
    expect(isTheName("viknan")).toBe(true);
    expect(isTheName("  Viknan ")).toBe(true);
    expect(isTheName("viknann")).toBe(false);
    expect(isTheName("nankiv")).toBe(false);
  });
});

describe("the wrong file", () => {
  it("recognises a résumé", () => {
    for (const name of [
      "Resume.pdf",
      "Jeya_Résumé_2026.pdf",
      "my CV.pdf",
      "cv_final.pdf",
      "Curriculum Vitae.pdf",
    ]) {
      expect(wrongFileReply(name)?.title, name).toBe(
        "That's a résumé, not a shortlist.",
      );
    }
  });

  it("recognises a photo", () => {
    for (const name of ["IMG_2041.HEIC", "selfie.jpg", "screenshot.png"]) {
      expect(wrongFileReply(name)?.title, name).toBe(
        "Nice picture. Still not a shortlist.",
      );
    }
  });

  it("leaves everything else to the plain message", () => {
    for (const name of ["offer.pdf", "notes.txt", "cvs.pdf", "recv.pdf"]) {
      expect(wrongFileReply(name), name).toBeNull();
    }
  });
});

describe("the 2 a.m. check", () => {
  const at = (h: number, m = 14) => new Date(2026, 9, 5, h, m);
  it("speaks only between one and five in the morning", () => {
    expect(lateNightLine(at(2))).toBe(
      "It's 2:14 AM. The list will say the same thing in the morning.",
    );
    expect(lateNightLine(at(1, 0))).toBe(
      "It's 1:00 AM. The list will say the same thing in the morning.",
    );
    expect(lateNightLine(at(4, 59))).not.toBeNull();
    for (const h of [0, 5, 9, 14, 23]) expect(lateNightLine(at(h))).toBeNull();
  });
});
