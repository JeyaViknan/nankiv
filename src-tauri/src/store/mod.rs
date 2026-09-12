//! Local persistence.
//!
//! A single SQLite database in the platform application-data directory. It never
//! leaves the machine, and the schema has no column for contact details, so
//! minimisation is enforced by structure rather than by discipline.

pub mod schema;

use crate::identity::name;
use crate::model::{Confidence, Identifier, KeyKind, NeoId, RegNo};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("this shortlist has already been imported")]
    DuplicateDrive { drive_id: i64, company: String },
}

pub struct Store {
    conn: Connection,
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    pub neo_id: Option<String>,
    pub reg_no: Option<String>,
    pub display_name: Option<String>,
    pub cohort: Option<String>,
    /// Off by default. Aggregates need no per-person disclosure.
    pub show_friend_cgpa: bool,
}

impl Profile {
    pub fn is_configured(&self) -> bool {
        self.neo_id.is_some() || self.reg_no.is_some()
    }

    /// Key kinds this profile can answer membership questions for. A file keyed
    /// by something absent here yields `Undetermined`, never `NotShortlisted`.
    pub fn known_kinds(&self) -> Vec<KeyKind> {
        let mut v = Vec::new();
        if self.neo_id.is_some() {
            v.push(KeyKind::NeoId);
        }
        if self.reg_no.is_some() {
            v.push(KeyKind::RegNo);
        }
        v
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub id: i64,
    pub label: String,
    pub neo_id: Option<String>,
    pub reg_no: Option<String>,
    pub group_tag: Option<String>,
}

/// The two academic facts nankiv stores about a student: nothing else has a
/// column to live in.
pub type AcademicFacts = (Option<f64>, Option<String>);

/// A deleted drive, held just long enough to undo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveSnapshot {
    pub company: String,
    pub drive_date: Option<String>,
    pub source_filename: String,
    pub content_hash: String,
    pub shape: String,
    pub primary_key: Option<String>,
    pub round_label: Option<String>,
    pub neo_ids: Vec<String>,
    pub reg_nos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveRecord {
    pub id: i64,
    pub company: String,
    pub drive_date: Option<String>,
    pub imported_at: String,
    pub source_filename: String,
    pub content_hash: String,
    pub shape: String,
    pub primary_key: Option<String>,
    pub total_students: usize,
    pub round_label: Option<String>,
    pub parent_drive_id: Option<i64>,
}

// ---------------------------------------------------------------------------

impl Store {
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        schema::migrate(&conn)?;
        Ok(Store { conn })
    }

    pub fn open_in_memory() -> Result<Store, StoreError> {
        let conn = Connection::open_in_memory()?;
        schema::migrate(&conn)?;
        Ok(Store { conn })
    }

    // --- profile ---------------------------------------------------------

    pub fn profile(&self) -> Result<Profile, StoreError> {
        let p = self
            .conn
            .query_row(
                "SELECT neo_id, reg_no, display_name, cohort, show_friend_cgpa
                 FROM profile WHERE id = 1",
                [],
                |r| {
                    Ok(Profile {
                        neo_id: r.get(0)?,
                        reg_no: r.get(1)?,
                        display_name: r.get(2)?,
                        cohort: r.get(3)?,
                        show_friend_cgpa: r.get::<_, i64>(4)? != 0,
                    })
                },
            )
            .optional()?;
        Ok(p.unwrap_or_default())
    }

    pub fn save_profile(&self, p: &Profile) -> Result<(), StoreError> {
        // Derive the cohort from the registration number when we have one, so
        // the analytics baseline is filtered to the student's own batch.
        let cohort = p.cohort.clone().or_else(|| {
            p.reg_no
                .as_deref()
                .and_then(RegNo::parse)
                .map(|r| r.admission_year().to_string())
        });
        self.conn.execute(
            "INSERT INTO profile (id, neo_id, reg_no, display_name, cohort, show_friend_cgpa, created_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                neo_id = excluded.neo_id,
                reg_no = excluded.reg_no,
                display_name = excluded.display_name,
                cohort = excluded.cohort,
                show_friend_cgpa = excluded.show_friend_cgpa",
            params![
                p.neo_id,
                p.reg_no,
                p.display_name,
                cohort,
                p.show_friend_cgpa as i64
            ],
        )?;
        Ok(())
    }

    // --- friends ---------------------------------------------------------

    pub fn add_friend(
        &self,
        label: &str,
        neo_id: Option<&str>,
        reg_no: Option<&str>,
        group_tag: Option<&str>,
    ) -> Result<i64, StoreError> {
        // Deduplicate explicitly rather than with ON CONFLICT: the unique
        // indexes are partial, and a single conflict target cannot cover both
        // identifier columns.
        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM friends
                 WHERE (?1 IS NOT NULL AND neo_id = ?1)
                    OR (?2 IS NOT NULL AND reg_no = ?2)
                 LIMIT 1",
                params![neo_id, reg_no],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE friends SET label = ?2,
                                    neo_id = COALESCE(?3, neo_id),
                                    reg_no = COALESCE(?4, reg_no),
                                    group_tag = ?5
                 WHERE id = ?1",
                params![id, label, neo_id, reg_no, group_tag],
            )?;
            return Ok(id);
        }

        self.conn.execute(
            "INSERT INTO friends (label, neo_id, reg_no, group_tag, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![label, neo_id, reg_no, group_tag],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn friends(&self) -> Result<Vec<Friend>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, neo_id, reg_no, group_tag FROM friends ORDER BY label COLLATE NOCASE",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Friend {
                    id: r.get(0)?,
                    label: r.get(1)?,
                    neo_id: r.get(2)?,
                    reg_no: r.get(3)?,
                    group_tag: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn remove_friend(&self, id: i64) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM friends WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- drives ----------------------------------------------------------

    /// Finds an existing drive with the same content hash.
    pub fn find_by_hash(&self, hash: &str) -> Result<Option<DriveRecord>, StoreError> {
        let r = self
            .conn
            .query_row(
                "SELECT id, company, drive_date, imported_at, source_filename, content_hash,
                        shape, primary_key, total_students, round_label, parent_drive_id
                 FROM drives WHERE content_hash = ?1",
                [hash],
                map_drive,
            )
            .optional()?;
        Ok(r)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_drive(
        &self,
        company: &str,
        drive_date: Option<&str>,
        filename: &str,
        hash: &str,
        shape: &str,
        primary_key: Option<&str>,
        neo_ids: &BTreeSet<NeoId>,
        reg_nos: &BTreeSet<RegNo>,
        round_label: Option<&str>,
    ) -> Result<i64, StoreError> {
        if let Some(existing) = self.find_by_hash(hash)? {
            return Err(StoreError::DuplicateDrive {
                drive_id: existing.id,
                company: existing.company,
            });
        }
        let total = if neo_ids.len() >= reg_nos.len() {
            neo_ids.len()
        } else {
            reg_nos.len()
        };
        self.conn.execute(
            "INSERT INTO drives (company, drive_date, imported_at, source_filename, content_hash,
                                 shape, primary_key, total_students, round_label)
             VALUES (?1, ?2, datetime('now'), ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                company,
                drive_date,
                filename,
                hash,
                shape,
                primary_key,
                total as i64,
                round_label
            ],
        )?;
        let id = self.conn.last_insert_rowid();

        for n in neo_ids {
            self.conn.execute(
                "INSERT OR IGNORE INTO drive_members (drive_id, neo_id, reg_no) VALUES (?1, ?2, '')",
                params![id, n.as_str()],
            )?;
        }
        for r in reg_nos {
            self.conn.execute(
                "INSERT OR IGNORE INTO drive_members (drive_id, neo_id, reg_no) VALUES (?1, '', ?2)",
                params![id, r.as_str()],
            )?;
        }
        Ok(id)
    }

    pub fn drives(&self) -> Result<Vec<DriveRecord>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company, drive_date, imported_at, source_filename, content_hash,
                    shape, primary_key, total_students, round_label, parent_drive_id
             FROM drives ORDER BY imported_at DESC, id DESC",
        )?;
        let rows = stmt
            .query_map([], map_drive)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn drive(&self, id: i64) -> Result<Option<DriveRecord>, StoreError> {
        let r = self
            .conn
            .query_row(
                "SELECT id, company, drive_date, imported_at, source_filename, content_hash,
                        shape, primary_key, total_students, round_label, parent_drive_id
                 FROM drives WHERE id = ?1",
                params![id],
                map_drive,
            )
            .optional()?;
        Ok(r)
    }

    pub fn delete_drive(&self, id: i64) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM drives WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Renames a drive.
    ///
    /// The company is inferred from the filename at import, and a filename is
    /// not a promise. Without this the first guess would be permanent.
    pub fn rename_drive(&self, id: i64, company: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE drives SET company = ?2 WHERE id = ?1",
            params![id, company.trim()],
        )?;
        Ok(())
    }

    /// Records that one drive is a later round of another.
    pub fn set_drive_parent(&self, id: i64, parent: Option<i64>) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE drives SET parent_drive_id = ?2 WHERE id = ?1",
            params![id, parent],
        )?;
        Ok(())
    }

    /// Everything needed to put a deleted drive back.
    ///
    /// Deleting is offered without a confirmation dialog, which is only humane
    /// if it can be undone. Identity edges are keyed by source file and live in
    /// their own table, so they survive the delete and need no restoring — what
    /// the student loses, and gets back, is the drive and its membership.
    pub fn snapshot_drive(&self, id: i64) -> Result<Option<DriveSnapshot>, StoreError> {
        let Some(d) = self.drive(id)? else {
            return Ok(None);
        };
        Ok(Some(DriveSnapshot {
            company: d.company,
            drive_date: d.drive_date,
            source_filename: d.source_filename,
            content_hash: d.content_hash,
            shape: d.shape,
            primary_key: d.primary_key,
            round_label: d.round_label,
            neo_ids: self
                .drive_neo_ids(id)?
                .into_iter()
                .map(|n| n.as_str().to_string())
                .collect(),
            reg_nos: self
                .drive_reg_nos(id)?
                .into_iter()
                .map(|r| r.as_str().to_string())
                .collect(),
        }))
    }

    /// Puts a snapshot back. Returns the new drive id.
    pub fn restore_drive(&self, snap: &DriveSnapshot) -> Result<i64, StoreError> {
        let neo: BTreeSet<NeoId> = snap.neo_ids.iter().filter_map(|s| NeoId::parse(s)).collect();
        let reg: BTreeSet<RegNo> = snap.reg_nos.iter().filter_map(|s| RegNo::parse(s)).collect();
        self.insert_drive(
            &snap.company,
            snap.drive_date.as_deref(),
            &snap.source_filename,
            &snap.content_hash,
            &snap.shape,
            snap.primary_key.as_deref(),
            &neo,
            &reg,
            snap.round_label.as_deref(),
        )
    }

    pub fn drive_neo_ids(&self, drive_id: i64) -> Result<BTreeSet<NeoId>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT neo_id FROM drive_members WHERE drive_id = ?1 AND neo_id != ''")?;
        let v = stmt
            .query_map(params![drive_id], |r| r.get::<_, String>(0))?
            .filter_map(|s| s.ok().and_then(|x| NeoId::parse(&x)))
            .collect();
        Ok(v)
    }

    pub fn drive_reg_nos(&self, drive_id: i64) -> Result<BTreeSet<RegNo>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT reg_no FROM drive_members WHERE drive_id = ?1 AND reg_no != ''")?;
        let v = stmt
            .query_map(params![drive_id], |r| r.get::<_, String>(0))?
            .filter_map(|s| s.ok().and_then(|x| RegNo::parse(&x)))
            .collect();
        Ok(v)
    }

    /// Whether a Neo ID appears in a drive. Exact membership, nothing inferred.
    pub fn drive_contains_neo(&self, drive_id: i64, neo: &str) -> Result<bool, StoreError> {
        let n: i64 = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM drive_members WHERE drive_id = ?1 AND neo_id = ?2)",
            params![drive_id, neo],
            |r| r.get(0),
        )?;
        Ok(n != 0)
    }

    pub fn drive_contains_reg(&self, drive_id: i64, reg: &str) -> Result<bool, StoreError> {
        let n: i64 = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM drive_members WHERE drive_id = ?1 AND reg_no = ?2)",
            params![drive_id, reg],
            |r| r.get(0),
        )?;
        Ok(n != 0)
    }

    // --- identity --------------------------------------------------------

    pub fn add_edge(
        &self,
        a: &Identifier,
        b: &Identifier,
        confidence: Confidence,
        source: &str,
    ) -> Result<(), StoreError> {
        let (ak, av) = ident_parts(a);
        let (bk, bv) = ident_parts(b);
        self.conn.execute(
            "INSERT OR IGNORE INTO identity_edges
                (a_kind, a_value, b_kind, b_value, confidence, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            params![ak, av, bk, bv, confidence.label(), source],
        )?;
        Ok(())
    }

    pub fn edges(&self) -> Result<Vec<(Identifier, Identifier, Confidence, String)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT a_kind, a_value, b_kind, b_value, confidence, source FROM identity_edges",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (ak, av, bk, bv, c, s) = row?;
            if let (Some(a), Some(b), Some(conf)) =
                (parse_ident(&ak, &av), parse_ident(&bk, &bv), parse_conf(&c))
            {
                out.push((a, b, conf, s));
            }
        }
        Ok(out)
    }

    pub fn remember_spelling(&self, key: &str, display: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO name_spellings (name_key, display) VALUES (?1, ?2)",
            params![key, display],
        )?;
        Ok(())
    }

    pub fn spellings(&self) -> Result<Vec<(String, String)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name_key, display FROM name_spellings")?;
        let v = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    }

    pub fn edge_count(&self) -> Result<usize, StoreError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM identity_edges", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    // --- academics -------------------------------------------------------

    pub fn upsert_academic(
        &self,
        reg: &RegNo,
        cgpa: Option<f64>,
        branch: Option<&str>,
        source: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO academics (reg_no, cgpa, branch, cohort, source)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(reg_no) DO UPDATE SET
                cgpa = COALESCE(excluded.cgpa, academics.cgpa),
                branch = COALESCE(excluded.branch, academics.branch)",
            params![reg.as_str(), cgpa, branch, reg.admission_year(), source],
        )?;
        Ok(())
    }

    pub fn academic(&self, reg: &str) -> Result<Option<AcademicFacts>, StoreError> {
        let r = self
            .conn
            .query_row(
                "SELECT cgpa, branch FROM academics WHERE reg_no = ?1",
                [reg],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(r)
    }

    pub fn academic_count(&self) -> Result<usize, StoreError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM academics", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    // --- baseline --------------------------------------------------------

    pub fn add_baseline(
        &self,
        cohort: &str,
        cgpa: Option<f64>,
        branch: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO baseline_values (cohort, cgpa, branch) VALUES (?1, ?2, ?3)",
            params![cohort, cgpa, branch],
        )?;
        Ok(())
    }

    /// The anonymous cohort baseline. When `cohort` is `None` every value is
    /// used, which is the right fallback for a student who has not told us
    /// their registration number.
    pub fn baseline(&self, cohort: Option<&str>) -> Result<(Vec<f64>, Vec<String>), StoreError> {
        let (sql, has_arg) = match cohort {
            Some(_) => (
                "SELECT cgpa, branch FROM baseline_values WHERE cohort = ?1",
                true,
            ),
            None => ("SELECT cgpa, branch FROM baseline_values", false),
        };
        let mut stmt = self.conn.prepare(sql)?;
        let mapper = |r: &rusqlite::Row| -> rusqlite::Result<(Option<f64>, Option<String>)> {
            Ok((r.get(0)?, r.get(1)?))
        };
        let rows: Vec<(Option<f64>, Option<String>)> = if has_arg {
            stmt.query_map(params![cohort.unwrap()], mapper)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], mapper)?.collect::<Result<Vec<_>, _>>()?
        };

        let mut cgpas = Vec::new();
        let mut branches = Vec::new();
        for (c, b) in rows {
            if let Some(c) = c {
                cgpas.push(c);
            }
            if let Some(b) = b {
                branches.push(b);
            }
        }
        Ok((cgpas, branches))
    }

    /// Removes everything. Backs the "wipe all data" control in settings.
    pub fn wipe(&self) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "DELETE FROM drive_members;
             DELETE FROM drives;
             DELETE FROM friends;
             DELETE FROM identity_edges;
             DELETE FROM name_spellings;
             DELETE FROM academics;
             DELETE FROM baseline_values;
             DELETE FROM profile;",
        )?;
        Ok(())
    }

    /// Counts for the settings data inventory.
    pub fn inventory(&self) -> Result<Vec<(String, usize)>, StoreError> {
        let mut out = Vec::new();
        for t in [
            "drives",
            "drive_members",
            "friends",
            "identity_edges",
            "academics",
            "baseline_values",
        ] {
            let n: i64 = self
                .conn
                .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))?;
            out.push((t.to_string(), n as usize));
        }
        Ok(out)
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

fn map_drive(r: &rusqlite::Row) -> rusqlite::Result<DriveRecord> {
    Ok(DriveRecord {
        id: r.get(0)?,
        company: r.get(1)?,
        drive_date: r.get(2)?,
        imported_at: r.get(3)?,
        source_filename: r.get(4)?,
        content_hash: r.get(5)?,
        shape: r.get(6)?,
        primary_key: r.get(7)?,
        total_students: r.get::<_, i64>(8)? as usize,
        round_label: r.get(9)?,
        parent_drive_id: r.get(10)?,
    })
}

fn ident_parts(i: &Identifier) -> (&'static str, String) {
    match i {
        Identifier::NeoId(x) => ("neo_id", x.as_str().to_string()),
        Identifier::RegNo(x) => ("reg_no", x.as_str().to_string()),
        Identifier::NameKey(x) => ("name", x.clone()),
    }
}

fn parse_ident(kind: &str, value: &str) -> Option<Identifier> {
    match kind {
        "neo_id" => NeoId::parse(value).map(Identifier::NeoId),
        "reg_no" => RegNo::parse(value).map(Identifier::RegNo),
        "name" => Some(Identifier::NameKey(name::name_key(value))),
        _ => None,
    }
}

fn parse_conf(s: &str) -> Option<Confidence> {
    match s {
        "verified" => Some(Confidence::Verified),
        "high" => Some(Confidence::High),
        "probable" => Some(Confidence::Probable),
        "unresolved" => Some(Confidence::Unresolved),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    fn neo(s: &str) -> NeoId {
        NeoId::parse(s).unwrap()
    }
    fn reg(s: &str) -> RegNo {
        RegNo::parse(s).unwrap()
    }

    #[test]
    fn profile_round_trips_and_derives_cohort() {
        let s = store();
        assert!(!s.profile().unwrap().is_configured());
        s.save_profile(&Profile {
            neo_id: Some("V9H0G6C4".into()),
            reg_no: Some("23BAI0002".into()),
            display_name: Some("Me".into()),
            cohort: None,
            show_friend_cgpa: false,
        })
        .unwrap();
        let p = s.profile().unwrap();
        assert!(p.is_configured());
        assert_eq!(p.cohort.as_deref(), Some("23"), "cohort from reg no");
        assert_eq!(p.known_kinds().len(), 2);
    }

    #[test]
    fn friend_cgpa_defaults_to_hidden() {
        let s = store();
        s.save_profile(&Profile {
            neo_id: Some("V9H0G6C4".into()),
            ..Default::default()
        })
        .unwrap();
        assert!(!s.profile().unwrap().show_friend_cgpa);
    }

    #[test]
    fn friends_round_trip_and_deduplicate_by_neo_id() {
        let s = store();
        s.add_friend("Arjun", Some("V9H0G6C4"), None, Some("core"))
            .unwrap();
        s.add_friend("Arjun R", Some("V9H0G6C4"), None, None)
            .unwrap();
        let f = s.friends().unwrap();
        assert_eq!(f.len(), 1, "same neo id must not duplicate");
        assert_eq!(f[0].label, "Arjun R", "label should update");
    }

    #[test]
    fn drive_insert_and_membership_is_exact() {
        let s = store();
        let ids: BTreeSet<NeoId> = ["V9H0G6C4", "C5U6K1E7"].iter().map(|x| neo(x)).collect();
        let id = s
            .insert_drive(
                "Siemens",
                None,
                "s.xlsx",
                "HASH1",
                "neo_id_only",
                Some("neo_id"),
                &ids,
                &BTreeSet::new(),
                None,
            )
            .unwrap();
        assert!(s.drive_contains_neo(id, "V9H0G6C4").unwrap());
        assert!(!s.drive_contains_neo(id, "T2D4R9N9").unwrap());
        assert_eq!(s.drive_neo_ids(id).unwrap().len(), 2);
        assert_eq!(s.drive(id).unwrap().unwrap().total_students, 2);
    }

    #[test]
    fn duplicate_import_is_reported_not_silently_added() {
        let s = store();
        let ids: BTreeSet<NeoId> = ["V9H0G6C4"].iter().map(|x| neo(x)).collect();
        let args = ("Siemens", "s.xlsx", "SAMEHASH", "neo_id_only");
        s.insert_drive(
            args.0,
            None,
            args.1,
            args.2,
            args.3,
            Some("neo_id"),
            &ids,
            &BTreeSet::new(),
            None,
        )
        .unwrap();
        let err = s
            .insert_drive(
                args.0,
                None,
                "renamed.xlsx",
                args.2,
                args.3,
                Some("neo_id"),
                &ids,
                &BTreeSet::new(),
                None,
            )
            .unwrap_err();
        match err {
            StoreError::DuplicateDrive { company, .. } => assert_eq!(company, "Siemens"),
            other => panic!("expected DuplicateDrive, got {other:?}"),
        }
        assert_eq!(s.drives().unwrap().len(), 1);
    }

    #[test]
    fn identity_edges_round_trip() {
        let s = store();
        let a = Identifier::NeoId(neo("E2S8L9L8"));
        let b = Identifier::RegNo(reg("23BCE1473"));
        s.add_edge(&a, &b, Confidence::Verified, "tredence")
            .unwrap();
        s.add_edge(&a, &b, Confidence::Verified, "tredence")
            .unwrap();
        assert_eq!(
            s.edge_count().unwrap(),
            1,
            "same claim should not duplicate"
        );
        let e = s.edges().unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].2, Confidence::Verified);
        assert_eq!(e[0].3, "tredence");
    }

    #[test]
    fn academics_merge_without_losing_known_values() {
        let s = store();
        let r = reg("23BAI0002");
        s.upsert_academic(&r, Some(8.24), Some("CSE"), "sheet")
            .unwrap();
        s.upsert_academic(&r, None, None, "other").unwrap();
        let a = s.academic("23BAI0002").unwrap().unwrap();
        assert_eq!(a.0, Some(8.24), "must not erase a known cgpa");
        assert_eq!(a.1.as_deref(), Some("CSE"));
    }

    #[test]
    fn baseline_filters_by_cohort() {
        let s = store();
        s.add_baseline("23", Some(8.5), Some("CSE")).unwrap();
        s.add_baseline("23", Some(9.0), Some("ECE")).unwrap();
        s.add_baseline("22", Some(7.0), Some("MECH")).unwrap();
        let (c23, b23) = s.baseline(Some("23")).unwrap();
        assert_eq!(c23.len(), 2);
        assert_eq!(b23.len(), 2);
        let (all, _) = s.baseline(None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn wipe_clears_everything() {
        let s = store();
        s.save_profile(&Profile {
            neo_id: Some("V9H0G6C4".into()),
            ..Default::default()
        })
        .unwrap();
        s.add_friend("A", Some("C5U6K1E7"), None, None).unwrap();
        s.add_baseline("23", Some(8.0), None).unwrap();
        s.wipe().unwrap();
        assert!(!s.profile().unwrap().is_configured());
        assert!(s.friends().unwrap().is_empty());
        assert!(s.baseline(None).unwrap().0.is_empty());
    }

    #[test]
    fn inventory_reports_what_is_stored() {
        let s = store();
        s.add_friend("A", Some("C5U6K1E7"), None, None).unwrap();
        let inv = s.inventory().unwrap();
        let friends = inv.iter().find(|(t, _)| t == "friends").unwrap();
        assert_eq!(friends.1, 1);
    }

    #[test]
    fn renaming_a_drive_sticks() {
        let s = store();
        let ids: BTreeSet<NeoId> = ["V9H0G6C4"].iter().map(|x| neo(x)).collect();
        let id = s
            .insert_drive(
                "Unnamed drive", None, "a.xlsx", "H", "neo_id_only",
                Some("neo_id"), &ids, &BTreeSet::new(), None,
            )
            .unwrap();
        s.rename_drive(id, "  Siemens SISW  ").unwrap();
        assert_eq!(s.drive(id).unwrap().unwrap().company, "Siemens SISW");
    }

    #[test]
    fn a_deleted_drive_can_be_restored_exactly() {
        // Undo has to be faithful, or it is worse than a confirmation dialog.
        let s = store();
        let ids: BTreeSet<NeoId> = ["V9H0G6C4", "C5U6K1E7"].iter().map(|x| neo(x)).collect();
        let regs: BTreeSet<RegNo> = ["23BAI0001"].iter().map(|x| reg(x)).collect();
        let id = s
            .insert_drive(
                "Siemens", Some("28-07-26"), "s.xlsx", "HASH", "linked",
                Some("neo_id"), &ids, &regs, Some("Round 1"),
            )
            .unwrap();

        let snap = s.snapshot_drive(id).unwrap().expect("snapshot");
        s.delete_drive(id).unwrap();
        assert!(s.drives().unwrap().is_empty());

        let new_id = s.restore_drive(&snap).unwrap();
        let d = s.drive(new_id).unwrap().unwrap();
        assert_eq!(d.company, "Siemens");
        assert_eq!(d.drive_date.as_deref(), Some("28-07-26"));
        assert_eq!(d.content_hash, "HASH");
        assert_eq!(d.round_label.as_deref(), Some("Round 1"));
        assert_eq!(s.drive_neo_ids(new_id).unwrap().len(), 2);
        assert_eq!(s.drive_reg_nos(new_id).unwrap().len(), 1);
        assert!(s.drive_contains_neo(new_id, "V9H0G6C4").unwrap());
    }

    #[test]
    fn restoring_frees_the_hash_so_it_is_not_a_duplicate() {
        let s = store();
        let ids: BTreeSet<NeoId> = ["V9H0G6C4"].iter().map(|x| neo(x)).collect();
        let id = s
            .insert_drive("X", None, "a.xlsx", "H", "neo_id_only", Some("neo_id"), &ids, &BTreeSet::new(), None)
            .unwrap();
        let snap = s.snapshot_drive(id).unwrap().unwrap();
        s.delete_drive(id).unwrap();
        // The unique content hash must have been released by the delete.
        assert!(s.restore_drive(&snap).is_ok());
    }

    #[test]
    fn snapshotting_a_missing_drive_yields_nothing() {
        assert!(store().snapshot_drive(999).unwrap().is_none());
    }

    #[test]
    fn rounds_can_be_linked_and_unlinked() {
        let s = store();
        let a: BTreeSet<NeoId> = ["V9H0G6C4"].iter().map(|x| neo(x)).collect();
        let b: BTreeSet<NeoId> = ["C5U6K1E7"].iter().map(|x| neo(x)).collect();
        let r1 = s.insert_drive("T", None, "1.xlsx", "H1", "neo_id_only", Some("neo_id"), &a, &BTreeSet::new(), None).unwrap();
        let r2 = s.insert_drive("T", None, "2.xlsx", "H2", "neo_id_only", Some("neo_id"), &b, &BTreeSet::new(), None).unwrap();

        s.set_drive_parent(r2, Some(r1)).unwrap();
        assert_eq!(s.drive(r2).unwrap().unwrap().parent_drive_id, Some(r1));

        s.set_drive_parent(r2, None).unwrap();
        assert_eq!(s.drive(r2).unwrap().unwrap().parent_drive_id, None);
    }

    #[test]
    fn deleting_a_parent_leaves_the_child_intact() {
        // ON DELETE SET NULL: removing round one must not take round two with it.
        let s = store();
        let a: BTreeSet<NeoId> = ["V9H0G6C4"].iter().map(|x| neo(x)).collect();
        let b: BTreeSet<NeoId> = ["C5U6K1E7"].iter().map(|x| neo(x)).collect();
        let r1 = s.insert_drive("T", None, "1.xlsx", "H1", "neo_id_only", Some("neo_id"), &a, &BTreeSet::new(), None).unwrap();
        let r2 = s.insert_drive("T", None, "2.xlsx", "H2", "neo_id_only", Some("neo_id"), &b, &BTreeSet::new(), None).unwrap();
        s.set_drive_parent(r2, Some(r1)).unwrap();

        s.delete_drive(r1).unwrap();
        let child = s.drive(r2).unwrap().expect("child survives");
        assert_eq!(child.parent_drive_id, None);
    }

    #[test]
    fn deleting_a_drive_cascades() {
        let s = store();
        let ids: BTreeSet<NeoId> = ["V9H0G6C4"].iter().map(|x| neo(x)).collect();
        let id = s
            .insert_drive(
                "X",
                None,
                "a.xlsx",
                "H",
                "neo_id_only",
                Some("neo_id"),
                &ids,
                &BTreeSet::new(),
                None,
            )
            .unwrap();
        s.delete_drive(id).unwrap();
        assert!(s.drive_neo_ids(id).unwrap().is_empty());
    }
}
