//! Database schema and migrations.
//!
//! Versioned from the first release, because the schema will move. Each
//! migration is applied inside a transaction and recorded, so a partially
//! applied upgrade cannot leave the database in an unknown state.

use rusqlite::{Connection, Result};

/// Ordered migrations. Append only — never edit one that has shipped.
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_initial",
        r#"
    -- The person using the application.
    CREATE TABLE profile (
        id              INTEGER PRIMARY KEY CHECK (id = 1),
        neo_id          TEXT,
        reg_no          TEXT,
        display_name    TEXT,
        cohort          TEXT,
        show_friend_cgpa INTEGER NOT NULL DEFAULT 0,
        created_at      TEXT NOT NULL
    );

    -- People the student tracks. Stored by identifier once confirmed, so a
    -- friend never has to be re-resolved from a name.
    CREATE TABLE friends (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        label       TEXT NOT NULL,
        neo_id      TEXT,
        reg_no      TEXT,
        group_tag   TEXT,
        created_at  TEXT NOT NULL,
        CHECK (neo_id IS NOT NULL OR reg_no IS NOT NULL)
    );
    CREATE UNIQUE INDEX idx_friends_neo ON friends(neo_id) WHERE neo_id IS NOT NULL;
    CREATE UNIQUE INDEX idx_friends_reg ON friends(reg_no) WHERE reg_no IS NOT NULL;

    -- One imported shortlist.
    CREATE TABLE drives (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        company         TEXT NOT NULL,
        drive_date      TEXT,
        imported_at     TEXT NOT NULL,
        source_filename TEXT NOT NULL,
        content_hash    TEXT NOT NULL UNIQUE,
        shape           TEXT NOT NULL,
        primary_key     TEXT,
        total_students  INTEGER NOT NULL,
        round_label     TEXT,
        parent_drive_id INTEGER REFERENCES drives(id) ON DELETE SET NULL
    );
    CREATE INDEX idx_drives_company ON drives(company);
    CREATE INDEX idx_drives_imported ON drives(imported_at DESC);

    -- Membership. The deterministic tier: exact identifiers, nothing inferred.
    CREATE TABLE drive_members (
        drive_id    INTEGER NOT NULL REFERENCES drives(id) ON DELETE CASCADE,
        neo_id      TEXT,
        reg_no      TEXT,
        PRIMARY KEY (drive_id, neo_id, reg_no)
    );
    CREATE INDEX idx_members_neo ON drive_members(neo_id);
    CREATE INDEX idx_members_reg ON drive_members(reg_no);

    -- Identity graph: nodes and typed edges.
    CREATE TABLE identity_edges (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        a_kind      TEXT NOT NULL,
        a_value     TEXT NOT NULL,
        b_kind      TEXT NOT NULL,
        b_value     TEXT NOT NULL,
        confidence  TEXT NOT NULL,
        source      TEXT NOT NULL,
        created_at  TEXT NOT NULL
    );
    CREATE UNIQUE INDEX idx_edges_unique
        ON identity_edges(a_kind, a_value, b_kind, b_value, source);

    -- Human-readable spelling behind a normalised name key.
    CREATE TABLE name_spellings (
        name_key    TEXT PRIMARY KEY,
        display     TEXT NOT NULL
    );

    -- Academic facts. Deliberately minimal: phone, email, date of birth,
    -- gender, resume links and 10th/12th marks are discarded at ingestion and
    -- have no column here to land in.
    CREATE TABLE academics (
        reg_no      TEXT PRIMARY KEY,
        cgpa        REAL,
        branch      TEXT,
        cohort      TEXT,
        source      TEXT NOT NULL
    );
    CREATE INDEX idx_academics_cohort ON academics(cohort);

    -- Anonymous cohort baseline: values only, no identifiers.
    CREATE TABLE baseline_values (
        id      INTEGER PRIMARY KEY AUTOINCREMENT,
        cohort  TEXT NOT NULL,
        cgpa    REAL,
        branch  TEXT
    );
    CREATE INDEX idx_baseline_cohort ON baseline_values(cohort);
    "#,
    ),
    (
        "0002_app_meta",
        r#"
    -- Small key/value store for things about the installation itself, as
    -- distinct from anything about a student. Currently just which version of
    -- the bundled reference pack has been seeded.
    CREATE TABLE app_meta (
        key     TEXT PRIMARY KEY,
        value   TEXT NOT NULL
    );
    "#,
    ),
];

/// Applies any migrations the database has not seen.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS schema_migrations (
             name        TEXT PRIMARY KEY,
             applied_at  TEXT NOT NULL
         );",
    )?;

    for (name, sql) in MIGRATIONS {
        let already: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name = ?1)",
            [name],
            |r| r.get(0),
        )?;
        if already {
            continue;
        }
        conn.execute_batch("BEGIN")?;
        match conn.execute_batch(sql) {
            Ok(()) => {
                conn.execute(
                    "INSERT INTO schema_migrations (name, applied_at) VALUES (?1, datetime('now'))",
                    [name],
                )?;
                conn.execute_batch("COMMIT")?;
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    #[test]
    fn migration_creates_every_table() {
        let c = mem();
        for t in [
            "profile",
            "friends",
            "drives",
            "drive_members",
            "identity_edges",
            "name_spellings",
            "academics",
            "baseline_values",
        ] {
            let n: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [t],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "missing table {t}");
        }
    }

    #[test]
    fn migration_is_idempotent() {
        let c = mem();
        migrate(&c).unwrap();
        migrate(&c).unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, MIGRATIONS.len() as i64);
    }

    #[test]
    fn duplicate_drive_hash_is_rejected_by_the_schema() {
        let c = mem();
        let ins = "INSERT INTO drives (company, imported_at, source_filename, content_hash, shape, total_students)
                   VALUES ('X', datetime('now'), 'a.xlsx', 'HASH', 'neo_id_only', 10)";
        c.execute(ins, []).unwrap();
        assert!(c.execute(ins, []).is_err(), "hash must be unique");
    }

    #[test]
    fn a_friend_needs_at_least_one_identifier() {
        let c = mem();
        let r = c.execute(
            "INSERT INTO friends (label, created_at) VALUES ('Nobody', datetime('now'))",
            [],
        );
        assert!(r.is_err(), "a friend with no identifier is meaningless");
    }

    #[test]
    fn deleting_a_drive_removes_its_members() {
        let c = mem();
        c.execute(
            "INSERT INTO drives (id, company, imported_at, source_filename, content_hash, shape, total_students)
             VALUES (1,'X',datetime('now'),'a.xlsx','H','neo_id_only',1)",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO drive_members (drive_id, neo_id, reg_no) VALUES (1,'V9H0G6C4','')",
            [],
        )
        .unwrap();
        c.execute("DELETE FROM drives WHERE id = 1", []).unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM drive_members", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "cascade should have removed members");
    }

    #[test]
    fn academics_table_has_no_column_for_contact_details() {
        let c = mem();
        let stmt = c.prepare("SELECT * FROM academics LIMIT 0").unwrap();
        let cols: Vec<String> = stmt
            .column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        for forbidden in [
            "phone", "email", "dob", "gender", "resume", "tenth", "twelfth",
        ] {
            assert!(
                !cols.iter().any(|c| c.contains(forbidden)),
                "schema must not be able to store {forbidden}"
            );
        }
        drop(stmt);
    }
}
