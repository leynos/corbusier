//! Shared `PostgreSQL` support helpers for tenant-scoped adapters.
//!
//! This facade keeps bounded contexts from depending directly on another
//! adapter module's internal layout while still reusing the canonical tenant
//! transaction and blocking-execution helpers.

pub(crate) use crate::message::adapters::postgres::tenant_tx::{
    FromTxError, TxError, ensure_tenant_exists, with_tenant_read_tx, with_tenant_tx,
};
