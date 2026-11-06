//! Regression coverage that a fresh cluster resolves the UTC time zone without
//! any repair of the installed time-zone data.

use super::{BoxError, PostgresCluster, postgres_cluster};
use diesel::{Connection, PgConnection, QueryableByName, RunQueryDsl};
use rstest::rstest;

/// Connects through the public cluster fixture with an explicit UTC setting.
///
/// `postgresql_archive` 0.22.0 extracts the bundle's hard-linked `UTC` alias,
/// so the cluster accepts `TimeZone=UTC` as installed.
#[rstest]
fn postgres_connection_accepts_utc(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    #[derive(QueryableByName)]
    struct TimezoneRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        timezone: String,
    }

    let cluster = postgres_cluster?;
    let url = format!(
        "{}?options=-c%20TimeZone%3DUTC",
        cluster.connection().database_url("postgres")
    );
    let mut connection = PgConnection::establish(&url)?;
    let row: TimezoneRow = diesel::sql_query("SELECT current_setting('TimeZone') AS timezone")
        .get_result(&mut connection)?;
    if row.timezone != "UTC" {
        return Err(format!("connection timezone: expected UTC, got {:?}", row.timezone).into());
    }
    Ok(())
}
