//! The widget snapshot, pinned as files.
//!
//! The snapshot is a contract between this core and code in other languages:
//! the macOS widget decodes it in Swift and the Windows widget binds it into
//! Adaptive Cards. Each scenario below builds a store the way the app would,
//! takes the snapshot the app would publish, and compares it with a file in
//! `widgets/fixtures`. The Swift tests and the card checks read the same files,
//! so a change to the contract fails here first, in review, rather than as a
//! blank widget on someone's desktop.
//!
//! Everything is synthetic — invented companies, invented identifiers, a
//! generated cohort — so the fixtures can live in a public repository.
//!
//! To accept an intentional change:
//!
//! ```sh
//! UPDATE_WIDGET_FIXTURES=1 cargo test --test widget_fixtures
//! ```

use nankiv_core::analytics::baseline::test_support::batch_like;
use nankiv_core::model::{NeoId, RegNo};
use nankiv_core::store::{Profile, Store};
use nankiv_core::widget::{self, Activity, WidgetSnapshot};
use std::collections::BTreeSet;
use std::path::PathBuf;
use time::macros::datetime;
use time::{Duration, OffsetDateTime};

const NOW: OffsetDateTime = datetime!(2026-09-17 15:00:00 UTC);

const YOU_NEO: &str = "A1B2C3D4";
const YOU_REG: &str = "23BCE1000";

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../widgets/fixtures")
}

// ---------------------------------------------------------------------------
// A synthetic cohort
// ---------------------------------------------------------------------------

struct Student {
    reg: String,
    cgpa: f64,
}

const PROGRAMMES: [(&str, &str); 4] = [
    ("BCE", "Computer Science and Engineering"),
    ("BAI", "Computer Science and Engineering (AI and ML)"),
    ("BEC", "Electronics and Communication Engineering"),
    ("BME", "Mechanical Engineering"),
];

/// 600 students whose CGPAs follow the measured batch shape, spread across
/// four programmes. Student 0 is the person using the app.
fn cohort() -> Vec<Student> {
    let values = batch_like();
    (0..600)
        .map(|i| {
            let (code, _) = PROGRAMMES[i % PROGRAMMES.len()];
            // Stride through the batch so every programme sees the full range.
            let cgpa = if i == 0 {
                8.87
            } else {
                values[(i * 37) % values.len()]
            };
            Student {
                reg: format!("23{code}{:04}", 1000 + i),
                cgpa,
            }
        })
        .collect()
}

fn regs<'a>(students: impl Iterator<Item = &'a Student>) -> BTreeSet<RegNo> {
    students.filter_map(|s| RegNo::parse(&s.reg)).collect()
}

/// Synthetic Neo IDs in the `LDLDLDLD` shape, none of which resolve to a
/// registration number — a file nankiv can answer but not analyse.
fn unlinked_neo_ids(n: usize) -> BTreeSet<NeoId> {
    let letters = b"EFGHJKLMNP";
    (0..n)
        .filter_map(|i| {
            let s = format!(
                "{}{}{}{}{}{}{}{}",
                letters[i % 10] as char,
                (i / 10) % 10,
                letters[(i / 3) % 10] as char,
                i % 7,
                letters[(i / 7) % 10] as char,
                (i / 2) % 10,
                letters[(i / 5) % 10] as char,
                i % 9,
            );
            NeoId::parse(&s)
        })
        .chain(NeoId::parse(YOU_NEO))
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Company {
    /// A clean CGPA bar at 8.5, computer science programmes only. You're in.
    Aurora,
    /// Everyone above 8.0 — which most of the batch is, so no filter can be
    /// told apart from chance. You're not in.
    Bluefin,
    /// No CGPA filter. You're in.
    Cedar,
    /// Keyed by Neo ID with no links to academic records: an answer, but no
    /// analysis. You're in.
    Driftwood,
    /// Skews high without a hard bar. You're not in.
    Elmstead,
}

fn insert(store: &Store, company: Company, people: &[Student]) -> i64 {
    let you = &people[0];
    let others = || people.iter().skip(1);
    let (name, file, set): (&str, &str, BTreeSet<RegNo>) = match company {
        Company::Aurora => (
            "Aurora Systems",
            "Aurora Systems shortlist.xlsx",
            regs(
                std::iter::once(you).chain(
                    others()
                        .filter(|s| {
                            s.cgpa >= 8.5 && (s.reg.contains("BCE") || s.reg.contains("BAI"))
                        })
                        .take(95),
                ),
            ),
        ),
        Company::Bluefin => (
            "Bluefin Analytics",
            "Bluefin Analytics - final list.xlsx",
            regs(others().filter(|s| s.cgpa >= 8.0).step_by(3).take(140)),
        ),
        Company::Cedar => (
            "Cedar Labs",
            "Cedar Labs.xlsx",
            regs(std::iter::once(you).chain(others().step_by(5).take(109))),
        ),
        Company::Driftwood => {
            let id = store
                .insert_drive(
                    "Driftwood Energy",
                    None,
                    "Driftwood Energy shortlist.csv",
                    "driftwood",
                    "neo_id_only",
                    Some("neo_id"),
                    &unlinked_neo_ids(59),
                    &BTreeSet::new(),
                    None,
                )
                .unwrap();
            return id;
        }
        Company::Elmstead => (
            "Elmstead Foods",
            "Elmstead Foods.xlsx",
            regs(
                others()
                    .filter(|s| s.cgpa >= 8.3 || (s.cgpa >= 7.2 && s.reg.ends_with('7')))
                    .step_by(2)
                    .take(120),
            ),
        ),
    };
    store
        .insert_drive(
            name,
            None,
            file,
            name,
            "reg_no_only",
            Some("reg_no"),
            &BTreeSet::new(),
            &set,
            None,
        )
        .unwrap()
}

struct Setup {
    profile: Option<Profile>,
    reference_data: bool,
    friends: bool,
    /// Oldest first; the last one is the latest drive.
    drives: &'static [Company],
}

const SEASON: &[Company] = &[
    Company::Elmstead,
    Company::Driftwood,
    Company::Cedar,
    Company::Bluefin,
    Company::Aurora,
];

fn you() -> Profile {
    Profile {
        neo_id: Some(YOU_NEO.into()),
        reg_no: Some(YOU_REG.into()),
        ..Default::default()
    }
}

fn build(setup: Setup, activity: impl FnOnce(&Store) -> Activity) -> WidgetSnapshot {
    let store = Store::open_in_memory().unwrap();
    let people = cohort();

    if let Some(p) = &setup.profile {
        store.save_profile(p).unwrap();
    }
    if setup.reference_data {
        for (i, s) in people.iter().enumerate() {
            let programme = PROGRAMMES[i % PROGRAMMES.len()].1;
            store
                .upsert_academic(
                    &RegNo::parse(&s.reg).unwrap(),
                    Some(s.cgpa),
                    Some(programme),
                    "fixture",
                )
                .unwrap();
        }
        for (i, v) in batch_like().into_iter().enumerate() {
            let programme = PROGRAMMES[i % PROGRAMMES.len()].1;
            store.add_baseline("23", Some(v), Some(programme)).unwrap();
        }
    }
    if setup.friends {
        // Labels a student would type, saved the ways students save them: most
        // by registration number, one by Neo ID only — which leaves that
        // friend undetermined in a registration-keyed file, as it should.
        let aurora_in: Vec<&Student> = people
            .iter()
            .skip(1)
            .filter(|s| s.cgpa >= 9.2 && s.reg.contains("BCE"))
            .collect();
        store
            .add_friend("Rahul", None, Some(&aurora_in[0].reg), None)
            .unwrap();
        store
            .add_friend("Meera", None, Some(&aurora_in[3].reg), None)
            .unwrap();
        let below = people.iter().skip(1).find(|s| s.cgpa < 8.0).unwrap();
        store
            .add_friend("Karthik", None, Some(&below.reg), None)
            .unwrap();
        store
            .add_friend("Priya", Some("Q7R8S9T0"), None, None)
            .unwrap();
        let mech = people
            .iter()
            .skip(1)
            .find(|s| s.reg.contains("BME") && s.cgpa >= 8.6)
            .unwrap();
        store
            .add_friend("Sam", None, Some(&mech.reg), None)
            .unwrap();
    }
    for company in setup.drives {
        insert(&store, *company, &people);
    }

    let activity = activity(&store);
    let mut snap = widget::build_snapshot(&store, activity, NOW).unwrap();

    // Import times come from the database clock. Pin them so the files are
    // stable: the latest three hours ago, each earlier one a day before that.
    for (k, d) in snap.drives.iter_mut().enumerate() {
        d.imported_at = widget::format_now(NOW - Duration::hours(3) - Duration::days(k as i64));
    }
    // Derived from the newest import, so it has to be pinned with them.
    snap.leads_until = snap
        .drives
        .first()
        .map(|_| widget::format_now(NOW - Duration::hours(3) + widget::RESULT_LEADS_FOR));
    snap
}

fn scenarios() -> Vec<(&'static str, WidgetSnapshot)> {
    let season = || Setup {
        profile: Some(you()),
        reference_data: true,
        friends: true,
        drives: SEASON,
    };
    vec![
        ("ready", build(season(), |_| Activity::Idle)),
        (
            "importing",
            build(season(), |_| {
                Activity::importing("Fjord Robotics shortlist.xlsx", NOW - Duration::seconds(20))
            }),
        ),
        (
            "failed",
            build(season(), |s| {
                widget::record_failure(s, "Round 2 - final.xlsx", NOW - Duration::hours(2))
                    .unwrap();
                widget::resting_activity(s)
            }),
        ),
        (
            "not_shortlisted",
            build(
                Setup {
                    drives: &[Company::Aurora, Company::Cedar, Company::Bluefin],
                    ..season()
                },
                |_| Activity::Idle,
            ),
        ),
        (
            "insufficient",
            build(
                Setup {
                    drives: &[Company::Bluefin, Company::Driftwood],
                    ..season()
                },
                |_| Activity::Idle,
            ),
        ),
        (
            "soft_preference",
            build(
                Setup {
                    drives: &[Company::Cedar, Company::Elmstead],
                    ..season()
                },
                |_| Activity::Idle,
            ),
        ),
        (
            "undetermined",
            build(
                Setup {
                    profile: Some(Profile {
                        neo_id: Some(YOU_NEO.into()),
                        ..Default::default()
                    }),
                    ..season()
                },
                |_| Activity::Idle,
            ),
        ),
        (
            "not_set_up",
            build(
                Setup {
                    reference_data: false,
                    friends: false,
                    ..season()
                },
                |_| Activity::Idle,
            ),
        ),
        (
            "no_identity",
            build(
                Setup {
                    profile: None,
                    friends: false,
                    drives: &[Company::Cedar, Company::Aurora],
                    ..season()
                },
                |_| Activity::Idle,
            ),
        ),
        (
            "empty",
            build(
                Setup {
                    friends: false,
                    drives: &[],
                    ..season()
                },
                |_| Activity::Idle,
            ),
        ),
    ]
}

fn render(snap: &WidgetSnapshot) -> String {
    serde_json::to_string_pretty(snap).unwrap() + "\n"
}

#[test]
fn fixtures_match_the_snapshot_the_app_publishes() {
    let dir = fixtures_dir();
    let update = std::env::var_os("UPDATE_WIDGET_FIXTURES").is_some();
    if update {
        std::fs::create_dir_all(&dir).unwrap();
    }

    // The link formats, so the Swift and Windows widgets build exactly the URLs
    // `parse_route` accepts.
    let links = serde_json::json!({
        "drive_42": widget::route_url(&widget::Route::Drive { id: 42 }),
        "latest": widget::route_url(&widget::Route::Latest),
        "shortlists": widget::route_url(&widget::Route::Shortlists),
    });
    for url in links.as_object().unwrap().values() {
        assert!(widget::parse_route(url.as_str().unwrap()).is_some());
    }
    let mut files = vec![(
        "links".to_string(),
        serde_json::to_string_pretty(&links).unwrap() + "\n",
    )];
    files.extend(
        scenarios()
            .into_iter()
            .map(|(name, snap)| (name.to_string(), render(&snap))),
    );

    let mut stale = Vec::new();
    for (name, expected) in files {
        let path = dir.join(format!("{name}.json"));
        if update {
            std::fs::write(&path, &expected).unwrap();
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            _ => stale.push(name),
        }
    }
    assert!(
        stale.is_empty(),
        "widget fixtures are out of date: {stale:?}\n\
         If the change is intended, run UPDATE_WIDGET_FIXTURES=1 cargo test --test widget_fixtures"
    );
}

#[test]
fn every_scenario_shows_what_it_is_named_for() {
    let all: std::collections::HashMap<_, _> = scenarios().into_iter().collect();
    let latest = |name: &str| all[name].drives.first().cloned();

    let ready = latest("ready").unwrap();
    assert_eq!(ready.company, "Aurora Systems");
    assert_eq!(ready.verdict.status, "shortlisted");
    assert_eq!(ready.analysis.status, "ready");
    let cutoff = ready.analysis.cutoff.as_ref().expect("a cutoff");
    assert_eq!(cutoff.kind, "hard_cutoff");
    assert_eq!(ready.circle.total, 5);
    assert!(ready.circle.shortlisted >= 1 && ready.circle.not_shortlisted >= 1);
    assert_eq!(
        ready.circle.undetermined, 1,
        "Priya is saved by Neo ID only"
    );

    assert!(matches!(
        all["importing"].activity,
        Activity::Importing { .. }
    ));
    assert!(matches!(all["failed"].activity, Activity::Failed { .. }));

    let no = latest("not_shortlisted").unwrap();
    assert_eq!(no.verdict.status, "not_shortlisted");
    assert_eq!(no.analysis.status, "ready");

    let thin = latest("insufficient").unwrap();
    assert_eq!(thin.verdict.status, "shortlisted");
    assert_eq!(thin.analysis.status, "insufficient");

    let soft = latest("soft_preference").unwrap();
    assert_eq!(soft.analysis.status, "ready");
    assert_ne!(
        soft.analysis.cutoff.as_ref().map(|c| c.kind.as_str()),
        Some("hard_cutoff")
    );

    let unknown = latest("undetermined").unwrap();
    assert_eq!(unknown.verdict.status, "undetermined");
    assert_eq!(unknown.verdict.reason.as_deref(), Some("needs_reg_no"));

    assert_eq!(latest("not_set_up").unwrap().analysis.status, "not_set_up");

    assert!(!all["no_identity"].identity_configured);
    assert_eq!(
        latest("no_identity").unwrap().verdict.reason.as_deref(),
        Some("no_identity_configured")
    );

    assert!(all["empty"].identity_configured);
    assert!(all["empty"].drives.is_empty());
}

#[test]
fn fixtures_carry_no_identifiers() {
    let people = cohort();
    for (name, snap) in scenarios() {
        let json = render(&snap);
        assert!(
            !json.contains(YOU_NEO) && !json.contains(YOU_REG),
            "{name} leaks your identifiers"
        );
        assert!(
            !people.iter().any(|s| json.contains(&s.reg)),
            "{name} leaks a registration number"
        );
        assert!(!json.contains("8.87"), "{name} leaks your CGPA");
    }
}

#[test]
fn a_full_season_stays_small() {
    // The widget decodes this on every timeline reload.
    for (name, snap) in scenarios() {
        let compact = serde_json::to_vec(&snap).unwrap();
        assert!(
            compact.len() < 32 * 1024,
            "{name} is {} bytes",
            compact.len()
        );
    }
}
