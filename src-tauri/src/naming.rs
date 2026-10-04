//! Naming a drive from what the file says about itself.
//!
//! A shortlist arrives as `HPE_SHORTLIST_01-10-2026.xlsx`, `list (1).xlsx`, or
//! a sheet whose first line reads "Siemens SISW – Shortlisted candidates for
//! Round 2". The name a student wants is the company, and the round they are
//! looking at. Three sources are tried in order — the filename, the title
//! lines above the table, the sheet name — and each is stripped of the words
//! every shortlist shares, of dates, years and download copy numbers, until
//! either a name is left or nothing is. The stage of the process the file
//! names — a test, an interview, a final list — becomes the round's label, so
//! a drive's rounds read `Test ✓ → Interview ✓` rather than `R1 → R2`.
//!
//! It is always a suggestion. The drive can be renamed in place, and a wrong
//! guess costs one click; a confident wrong guess that could not be changed
//! would cost more. These rules were checked against a real season's sixty
//! filenames, which is where most of the cases in the tests come from.

use once_cell::sync::Lazy;
use regex::Regex;

/// What could be read off a file about the drive it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inferred {
    pub company: String,
    /// As written, e.g. `28-07-26` or `25-Sep-2026`.
    pub date: Option<String>,
    /// The stage this list is for: "Test", "Interview", "Final", "R2"…
    pub round: Option<String>,
}

pub const UNNAMED: &str = "Unnamed drive";

/// Words every shortlist shares, and words that only describe its stage.
const NOISE: &[&str] = &[
    "shortlist",
    "shortlists",
    "shortlisted",
    "short",
    "listed",
    "list",
    "lists",
    "candidates",
    "candidate",
    "students",
    "student",
    "final",
    "offer",
    "offers",
    "selected",
    "selection",
    "selections",
    "result",
    "results",
    "interview",
    "interviews",
    "test",
    "tests",
    "online",
    "assessment",
    "oa",
    "round",
    "aptitude",
    "coding",
    "ppt",
    "gd",
    "hr",
    "additional",
    "additonal",
    "with",
    "neo",
    "id",
    "ids",
    "neoid",
    "reg",
    "regno",
    "register",
    "registration",
    "number",
    "numbers",
    "no",
    "the",
    "batch",
    "vit",
    "vellore",
    "institute",
    "technology",
    "placement",
    "placements",
    "drive",
    "campus",
    "hiring",
    "recruitment",
    "copy",
    "new",
    "set",
    "sets",
    "next",
    "stage",
    "level",
    "slot",
    "opt",
    "pasted",
    "jan",
    "feb",
    "mar",
    "apr",
    "may",
    "jun",
    "jul",
    "aug",
    "sep",
    "sept",
    "oct",
    "nov",
    "dec",
];

/// Small words that join a name rather than being one. Kept in lower case
/// inside a name, and dropped when one is left hanging at either end.
const CONNECTORS: &[&str] = &[
    "and", "or", "of", "on", "in", "at", "to", "by", "for", "via", "&",
];

/// The stages of a placement process, latest first: a file naming more than
/// one ("PPT and interviews") is for the later.
const STAGES: &[(&str, &str)] = &[
    (r"final|offers?", "Final"),
    (r"interviews?", "Interview"),
    (r"hr", "HR"),
    (r"gd|group[\s_-]*discussion", "GD"),
    (
        r"tests?|oa|online[\s_-]*assessment|assessment|aptitude|coding",
        "Test",
    ),
    (r"ppt", "PPT"),
];

// Edges are spelled out rather than written `\b`, which counts `_` as part of
// a word — and filenames are joined with underscores.
static STAGE_RES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    STAGES
        .iter()
        .map(|(pat, label)| {
            let re = Regex::new(&format!(r"(?i)(?:^|[^a-z])(?:{pat})(?:$|[^a-z])"))
                .expect("static pattern");
            (re, *label)
        })
        .collect()
});
static ROUND: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z])(?:round|r)[\s_-]*(\d)(?:$|[^\d])").expect("static pattern")
});
/// "Results batch 2": a second announcement, which is how a student tells two
/// lists from the same company apart.
static BATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|[^a-z])batch[\s_-]*(\d)(?:$|[^\d])").expect("static pattern")
});
static NUMERIC_DATE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[^\d])(\d{1,2})[-_./](\d{1,2})[-_./](\d{2,4})(?:$|[^\d])")
        .expect("static pattern")
});
static WORDED_DATE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|[^\d])(\d{1,2})[-_ ]?(jan|feb|mar|apr|may|jun|jul|aug|sept?|oct|nov|dec)[a-z]*[-_ ]?(\d{2,4})?")
        .expect("static pattern")
});
static GENERIC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^(sheet|book|workbook|untitled|document|data|export|download|file|table)\s*\d*$",
    )
    .expect("static pattern")
});
static SEPARATORS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[_\-–—.,:;|/()\[\]{}+]+").expect("static pattern"));
/// Years, ordinals ("27th", "03rd", and the "1sst" a hurried hand types), days
/// written with a month ("05oct"), and the copy numbers a browser appends.
static NUMBERISH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\d+[a-z]{0,4}$").expect("static pattern"));

/// Names a drive from its filename, the title lines above its table, and its
/// sheet names, in that order of preference.
pub fn infer(filename: &str, titles: &[String], sheets: &[String]) -> Inferred {
    // Files dropped on the window before their names were decoded were stored
    // as `Fractal%20test%20shortlist.xlsx`.
    let filename = crate::desktop::percent_decode(filename);
    let stem = filename
        .rsplit_once('.')
        .map(|(a, _)| a)
        .unwrap_or(&filename);

    let sources: Vec<&str> = std::iter::once(stem)
        .chain(titles.iter().map(String::as_str))
        .collect();
    let date = sources.iter().find_map(|s| date_in(s));
    let round = sources.iter().find_map(|s| round_of(s));

    let from_file = company_in(stem);
    let from_title = titles.iter().find_map(|t| company_in(t));
    let from_sheet = sheets.iter().find_map(|s| company_in(s));

    // A short filename ("gs.xlsx") is a weaker signal than a title that names
    // the company outright; a full one is the student's own label and wins.
    let company = match (from_file, from_title) {
        (Some(f), _) if letters(&f) >= 4 => f,
        (_, Some(t)) => t,
        (Some(f), None) => f,
        (None, None) => from_sheet.unwrap_or_else(|| UNNAMED.to_string()),
    };

    Inferred {
        company,
        date,
        round,
    }
}

/// The stage a piece of text names: "R2" when it numbers the round, otherwise
/// the latest stage it mentions.
pub fn round_of(text: &str) -> Option<String> {
    if let Some(c) = ROUND.captures(text) {
        return Some(format!("R{}", &c[1]));
    }
    STAGE_RES
        .iter()
        .find(|(re, _)| re.is_match(text))
        .map(|(_, label)| label.to_string())
        .or_else(|| BATCH.captures(text).map(|c| format!("Batch {}", &c[1])))
}

/// A real date in the text, if there is one: a time such as `20_42_02` has the
/// same shape and is not.
fn date_in(text: &str) -> Option<String> {
    for c in NUMERIC_DATE.captures_iter(text) {
        let (day, month) = (c[1].parse::<u32>().ok()?, c[2].parse::<u32>().ok()?);
        if (1..=31).contains(&day) && (1..=12).contains(&month) {
            return Some(format!("{}-{}-{}", &c[1], &c[2], &c[3]));
        }
    }
    WORDED_DATE.captures(text).map(|c| {
        let month = title(&c[2].to_lowercase());
        match c.get(3) {
            Some(year) => format!("{}-{month}-{}", &c[1], year.as_str()),
            None => format!("{} {month}", &c[1]),
        }
    })
}

/// What is left of a piece of text once everything that names nothing is gone.
fn company_in(text: &str) -> Option<String> {
    let text = NUMERIC_DATE.replace_all(text, " ");
    let text = ROUND.replace_all(&text, " ");
    let text = SEPARATORS.replace_all(&text, " ");
    if GENERIC.is_match(text.trim()) {
        return None;
    }
    // A name typed entirely in capitals is shouting, not a run of acronyms —
    // but one word in capitals ("RFPIO") is an acronym.
    let shouted = text.split_whitespace().count() > 1
        && text.chars().any(char::is_alphabetic)
        && !text.chars().any(char::is_lowercase);

    let mut words: Vec<String> = text
        .split_whitespace()
        .filter(|w| !NUMBERISH.is_match(w))
        .filter(|w| !NOISE.contains(&w.to_lowercase().as_str()))
        .map(|w| cased(w, shouted))
        .collect();
    while words.first().is_some_and(|w| is_connector(w)) {
        words.remove(0);
    }
    while words.last().is_some_and(|w| is_connector(w)) {
        words.pop();
    }
    let name = words.join(" ");
    (letters(&name) >= 2).then_some(name)
}

/// Short words that are words, not acronyms, so they are not shouted.
const PLAIN_SHORT: &[&str] = &[
    "non", "not", "all", "one", "two", "app", "web", "pro", "top", "big", "day",
];

fn is_connector(word: &str) -> bool {
    CONNECTORS.contains(&word.to_lowercase().as_str())
}

/// Restores the casing a company would use, as far as it can be guessed:
/// joining words stay small, a short lower-case word is an acronym (hpe →
/// HPE), a longer one a name (amazon → Amazon), shouting is lowered
/// (ACCENTURE COMMUNICATION → Accenture Communication), and anything already
/// mixed — BlackRock, PharmaAce — is left exactly as written.
fn cased(word: &str, shouted: bool) -> String {
    if is_connector(word) {
        return word.to_lowercase();
    }
    let has_lower = word.chars().any(char::is_lowercase);
    let has_upper = word.chars().any(char::is_uppercase);
    let n = word.chars().filter(|c| c.is_alphabetic()).count();
    match (has_lower, has_upper) {
        (true, false) if PLAIN_SHORT.contains(&word) => title(word),
        (true, false) if n <= 3 || word.contains('&') => word.to_uppercase(),
        (true, false) => title(word),
        (false, true) if shouted && n >= 5 => title(&word.to_lowercase()),
        _ => word.to_string(),
    }
}

fn title(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn letters(s: &str) -> usize {
    s.chars().filter(|c| c.is_alphabetic()).count()
}

/// The name the rules before this module gave a file.
///
/// Kept only to recognise drives still carrying one, so they can be renamed
/// once by the rules above without touching any name a student chose.
pub fn legacy_name(filename: &str) -> String {
    let stem = filename
        .rsplit_once('.')
        .map(|(a, _)| a)
        .unwrap_or(filename);
    let mut name = stem.to_string();
    for pat in [
        "shortlisted list",
        "shortlist",
        "shortlisted",
        "test",
        "interview",
        "additonal",
        "additional",
        "with neo id",
        "list",
    ] {
        let re = regex::RegexBuilder::new(&regex::escape(pat))
            .case_insensitive(true)
            .build()
            .expect("static pattern");
        name = re.replace_all(&name, " ").to_string();
    }
    for (pat, with) in [
        (r"\d{1,2}[-_]\d{1,2}[-_]\d{2,4}", " "),
        (r"\((\d+)\)", " "),
        (r"[_\-]+", " "),
        (r"\s+", " "),
    ] {
        name = Regex::new(pat)
            .expect("static pattern")
            .replace_all(&name, with)
            .to_string();
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        UNNAMED.to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(file: &str) -> String {
        infer(file, &[], &["Sheet1".into()]).company
    }

    #[test]
    fn a_real_seasons_filenames_name_their_companies() {
        let cases = [
            ("Tredence shortlisted list.xlsx", "Tredence"),
            ("EY GDS TEST SHORTLIST NEO ID.xlsx", "EY GDS"),
            ("smart test 16.09.2026.xlsx", "Smart"),
            ("amazon shortlist 2027.xlsx", "Amazon"),
            ("Embitel online assessment shortlist.xlsx", "Embitel"),
            ("Accenture 1sst set.xlsx", "Accenture"),
            ("Responsive%20shortlist.xlsx", "Responsive"),
            ("Fractal%20test%20shortlist%202.xlsx", "Fractal"),
            (
                "Deloitte shortlist  (consultative offerings).xlsx",
                "Deloitte Consultative Offerings",
            ),
            ("RFPIO.xlsx", "RFPIO"),
            ("Embitel non eligible students.xlsx", "Embitel Non Eligible"),
            (
                "Infosys_selected_students_dse&spe-1 2.xlsx",
                "Infosys DSE&SPE",
            ),
            (
                "OJ Commerce - Opt-in List_25-Sep-2026 20_42_02.xlsx",
                "OJ Commerce",
            ),
            (
                "Deloitte shortlist HCE profile.xlsx",
                "Deloitte HCE Profile",
            ),
            ("Infosys_results_batch-2 (1).xlsx", "Infosys"),
            ("gs.xlsx", "GS"),
            ("ama.xlsx", "AMA"),
            ("Infosys_interviews_03rd & 5th oct.xlsx", "Infosys"),
            (
                "Schneider Electric ppt and interviews on 05oct.xlsx",
                "Schneider Electric",
            ),
            ("ET&P Oracle shortlist.xlsx", "ET&P Oracle"),
            ("PharmaAce Additional test shortlist.xlsx", "PharmaAce"),
            ("HPE_SHORTLIST_01-10-2026.xlsx", "HPE"),
            ("ACCENTURE COMMUNICATION.xlsx", "Accenture Communication"),
            (
                "Hackathon 2026 - Caterpillar team shortlist.xlsx",
                "Hackathon Caterpillar Team",
            ),
            ("BlackRock - Test Shortlist.xlsx", "BlackRock"),
            ("tcs_shortlist_27th_cognizant.xlsx", "TCS Cognizant"),
            ("siemens_sisw_shortlist_2027_1.xlsx", "Siemens Sisw"),
        ];
        for (file, expected) in cases {
            assert_eq!(name(file), expected, "for {file}");
        }
    }

    #[test]
    fn the_stage_a_file_names_becomes_its_round() {
        let round = |f: &str| infer(f, &[], &[]).round;
        assert_eq!(
            round("Embitel online assessment shortlist.xlsx").as_deref(),
            Some("Test")
        );
        assert_eq!(
            round("Embitel interview shortlist.xlsx").as_deref(),
            Some("Interview")
        );
        assert_eq!(
            round("EY GDS TEST SHORTLIST NEO ID.xlsx").as_deref(),
            Some("Test")
        );
        // "PPT and interviews": the later stage.
        assert_eq!(
            round("Schneider Electric ppt and interviews on 05oct.xlsx").as_deref(),
            Some("Interview")
        );
        assert_eq!(round("Amazon round 2.xlsx").as_deref(), Some("R2"));
        assert_eq!(
            round("Infosys_results_batch-2 (1).xlsx").as_deref(),
            Some("Batch 2")
        );
        // A batch year is not a batch number.
        assert_eq!(round("VIT 2027 batch shortlist.xlsx"), None);
        assert_eq!(round("amazon_r3.xlsx").as_deref(), Some("R3"));
        assert_eq!(round("Amazon final list.xlsx").as_deref(), Some("Final"));
        assert_eq!(round("Tredence shortlisted list.xlsx"), None);
        // Letters inside a word are not a stage: "Deloitte HCE" has no HR.
        assert_eq!(round("Deloitte HCE shortlist.xlsx"), None);
    }

    #[test]
    fn dates_are_read_and_times_are_not() {
        let date = |f: &str| infer(f, &[], &[]).date;
        assert_eq!(
            date("Zluri Shortlist 28_07_26 (1).xlsx").as_deref(),
            Some("28-07-26")
        );
        assert_eq!(
            date("smart test 16.09.2026.xlsx").as_deref(),
            Some("16-09-2026")
        );
        assert_eq!(
            date("OJ Commerce - Opt-in List_25-Sep-2026 20_42_02.xlsx").as_deref(),
            Some("25-Sep-2026")
        );
        assert_eq!(
            date("Schneider Electric ppt and interviews on 05oct.xlsx").as_deref(),
            Some("05 Oct")
        );
        assert_eq!(date("Tredence shortlisted list.xlsx"), None);
    }

    #[test]
    fn a_generic_filename_falls_back_to_the_title() {
        let titles = vec!["VIT 2027 Batch – Goldman Sachs: Shortlisted Students (Round 2)".into()];
        for file in [
            "Shortlisted list (1).xlsx",
            "list.xlsx",
            "Book1.xlsx",
            "download.xlsx",
        ] {
            let i = infer(file, &titles, &["Sheet1".into()]);
            assert_eq!(i.company, "Goldman Sachs", "for {file}");
            assert_eq!(i.round.as_deref(), Some("R2"));
        }
    }

    #[test]
    fn a_title_that_names_the_company_beats_an_initialism() {
        let titles = vec!["Goldman Sachs – Shortlist".into()];
        assert_eq!(infer("gs.xlsx", &titles, &[]).company, "Goldman Sachs");
        assert_eq!(infer("gs.xlsx", &[], &[]).company, "GS");
    }

    #[test]
    fn the_sheet_name_is_the_last_resort_and_a_default_one_is_ignored() {
        assert_eq!(
            infer("shortlist.xlsx", &[], &["Zoho".into()]).company,
            "Zoho"
        );
        assert_eq!(
            infer("shortlist.xlsx", &[], &["Sheet1".into(), "Sheet 2".into()]).company,
            UNNAMED
        );
    }

    #[test]
    fn nothing_left_means_unnamed_not_a_guess() {
        assert_eq!(name("shortlist.xlsx"), UNNAMED);
        assert_eq!(name("Test shortlist.xlsx"), UNNAMED);
        assert_eq!(name("Final shortlisted list 2027.xlsx"), UNNAMED);
    }

    #[test]
    fn the_old_rules_are_recognised_exactly() {
        // What the drives imported before this module were called.
        for (file, old) in [
            (
                "Infosys_interviews_03rd & 5th oct.xlsx",
                "Infosys s 03rd & 5th oct",
            ),
            ("Fractal%20test%20shortlist%202.xlsx", "Fractal%20 %20 %202"),
            ("hpe shortlisted list.xlsx", "hpe"),
            ("Test shortlist.xlsx", UNNAMED),
            ("Zluri Shortlist 28_07_26 (1).xlsx", "Zluri"),
        ] {
            assert_eq!(legacy_name(file), old, "for {file}");
        }
    }
}
