use model::Tenant;
use tokio_postgres::Row;

pub(crate) fn from_row(row: Row) -> Tenant {
    Tenant {
        uid: row.get("uid"),
        host: row.get("host"),
        default: row.get("is_default"),
        title: row.get("title"),
        logo: row.get("logo"),
        favicon: row.get("favicon"),
        invalidation_flow: row.get("invalidation_flow"),
        authentication_flow: row.get("authentication_flow"),
        authorization_flow: row.get("authorization_flow"),
        enrollment_flow: row.get("enrollment_flow"),
        recovery_flow: row.get("recovery_flow"),
        unenrollment_flow: row.get("unenrollment_flow"),
        configuration_flow: row.get("configuration_flow"),
    }
}
